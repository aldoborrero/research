use clap::Parser;
use std::io::{self, Read};
use std::path::PathBuf;

use mdformat::config::{Config, LineEnding};
use mdformat::plugin::FormatterBuilder;
use mdformat::plugins::gfm::GfmPlugin;
use mdformat::plugins::mkdocs::config::MkDocsConfig;
use mdformat::plugins::mkdocs::MkDocsPlugin;

/// A CommonMark-compliant, opinionated Markdown formatter.
#[derive(Parser)]
#[command(name = "mdformat", version, about)]
struct Cli {
    /// Input files to format. Reads from stdin if none provided.
    files: Vec<PathBuf>,

    /// Line ending style.
    #[arg(long, value_name = "STYLE", value_parser = parse_line_ending)]
    end_of_line: Option<LineEnding>,

    /// Enable GFM (GitHub Flavored Markdown) extensions.
    #[arg(long)]
    gfm: bool,

    /// Enable MkDocs extensions.
    #[arg(long)]
    mkdocs: bool,

    /// Align semantic breaks in lists (MkDocs plugin).
    #[arg(long)]
    align_semantic_breaks_in_lists: bool,

    /// Don't escape undefined link references (MkDocs plugin).
    #[arg(long)]
    ignore_missing_references: bool,

    /// Disable math/LaTeX handling (MkDocs plugin).
    #[arg(long)]
    no_mkdocs_math: bool,
}

fn parse_line_ending(s: &str) -> Result<LineEnding, String> {
    match s {
        "lf" => Ok(LineEnding::Lf),
        "crlf" => Ok(LineEnding::CrLf),
        _ => Err(format!("invalid line ending: \"{s}\", expected \"lf\" or \"crlf\"")),
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Determine working directory for config file discovery
    let work_dir = if let Some(first) = cli.files.first() {
        first
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    } else {
        std::env::current_dir().unwrap_or_default()
    };

    // Load config file (if any)
    let mut config = Config::from_directory(&work_dir).unwrap_or_default();

    // CLI overrides for core config
    if let Some(eol) = cli.end_of_line {
        config.line_ending = eol;
    }

    // CLI overrides for MkDocs plugin config
    if cli.align_semantic_breaks_in_lists {
        config.merge_plugin_value(
            "mkdocs",
            "align_semantic_breaks_in_lists",
            toml::Value::Boolean(true),
        );
    }
    if cli.ignore_missing_references {
        config.merge_plugin_value(
            "mkdocs",
            "ignore_missing_references",
            toml::Value::Boolean(true),
        );
    }
    if cli.no_mkdocs_math {
        config.merge_plugin_value(
            "mkdocs",
            "no_mkdocs_math",
            toml::Value::Boolean(true),
        );
    }

    // Build formatter with plugins
    let mut builder = FormatterBuilder::new().config(config.clone());

    if cli.gfm {
        builder = builder.parser_extension(GfmPlugin::new());
    }

    if cli.mkdocs {
        let mkdocs_config: MkDocsConfig = config
            .plugin_config("mkdocs")
            .unwrap_or_default();
        builder = builder.parser_extension(MkDocsPlugin::new(mkdocs_config));
    }

    if cli.files.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let output = builder.format_str(&input);
        print!("{output}");
    } else {
        for path in &cli.files {
            let input = std::fs::read_to_string(path)?;
            let output = builder.format_str(&input);
            std::fs::write(path, &output)?;
        }
    }

    Ok(())
}

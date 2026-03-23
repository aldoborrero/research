use clap::Parser;
use std::io::{self, Read};

/// A CommonMark-compliant, opinionated Markdown formatter.
#[derive(Parser)]
#[command(name = "mdformat", version, about)]
struct Cli {
    /// Input files to format. Reads from stdin if none provided.
    files: Vec<String>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if cli.files.is_empty() {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let output = mdformat::format_str(&input);
        print!("{output}");
    } else {
        for path in &cli.files {
            let input = std::fs::read_to_string(path)?;
            let output = mdformat::format_str(&input);
            std::fs::write(path, &output)?;
        }
    }

    Ok(())
}

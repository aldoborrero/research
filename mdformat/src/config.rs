/// Formatter configuration options.
#[derive(Debug, Clone)]
pub struct Config {
    /// Line ending style.
    pub line_ending: LineEnding,
}

/// Line ending normalization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// Unix-style line endings (`\n`).
    Lf,
    /// Windows-style line endings (`\r\n`).
    CrLf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::Lf,
        }
    }
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "ewc",
    about = "Enhanced Word Count - A modern alternative to wc",
    version
)]
pub struct Args {
    /// Files to process
    #[arg(value_name = "FILE")]
    pub files: Vec<String>,

    /// Show line count only
    #[arg(short = 'l', long)]
    pub lines: bool,

    /// Show word count only
    #[arg(short = 'w', long)]
    pub words: bool,

    /// Show byte count only
    #[arg(short = 'c', long)]
    pub bytes: bool,

    /// Show longest line length
    #[arg(short = 'L', long)]
    pub max_line_length: bool,

    /// Disable colors and icons
    #[arg(long)]
    pub no_color: bool,

    /// Include hidden files and directories
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Compact one-line output format
    #[arg(short = 'C', long)]
    pub compact: bool,

    /// Show file list for directories
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

impl Args {
    pub fn show_lines(&self) -> bool {
        self.lines || self.show_all()
    }

    pub fn show_words(&self) -> bool {
        self.words || self.show_all()
    }

    pub fn show_bytes(&self) -> bool {
        self.bytes || self.show_all()
    }

    pub fn show_max_line_length(&self) -> bool {
        self.max_line_length
    }

    fn show_all(&self) -> bool {
        !self.lines && !self.words && !self.bytes && !self.max_line_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_args() -> Args {
        Args {
            files: vec![],
            lines: false,
            words: false,
            bytes: false,
            max_line_length: false,
            no_color: false,
            all: false,
            compact: false,
            verbose: false,
            json: false,
        }
    }

    #[test]
    fn default_shows_all() {
        let args = default_args();
        assert!(args.show_lines());
        assert!(args.show_words());
        assert!(args.show_bytes());
    }

    #[test]
    fn lines_only() {
        let args = Args {
            lines: true,
            ..default_args()
        };
        assert!(args.show_lines());
        assert!(!args.show_words());
        assert!(!args.show_bytes());
    }

    #[test]
    fn words_only() {
        let args = Args {
            words: true,
            ..default_args()
        };
        assert!(!args.show_lines());
        assert!(args.show_words());
        assert!(!args.show_bytes());
    }

    #[test]
    fn bytes_only() {
        let args = Args {
            bytes: true,
            ..default_args()
        };
        assert!(!args.show_lines());
        assert!(!args.show_words());
        assert!(args.show_bytes());
    }

    #[test]
    fn lines_and_words() {
        let args = Args {
            lines: true,
            words: true,
            ..default_args()
        };
        assert!(args.show_lines());
        assert!(args.show_words());
        assert!(!args.show_bytes());
    }

    #[test]
    fn no_color_flag_parsed() {
        let args = Args {
            no_color: true,
            ..default_args()
        };
        assert!(args.no_color);
    }

    #[test]
    fn all_flag_parsed() {
        let args = Args {
            all: true,
            ..default_args()
        };
        assert!(args.all);
    }

    #[test]
    fn compact_flag_parsed() {
        let args = Args {
            compact: true,
            ..default_args()
        };
        assert!(args.compact);
    }

    #[test]
    fn verbose_flag_parsed() {
        let args = Args {
            verbose: true,
            ..default_args()
        };
        assert!(args.verbose);
    }

    #[test]
    fn json_flag_parsed() {
        let args = Args {
            json: true,
            ..default_args()
        };
        assert!(args.json);
    }

    #[test]
    fn max_line_length_flag_parsed() {
        let args = Args {
            max_line_length: true,
            ..default_args()
        };
        assert!(args.max_line_length);
        assert!(args.show_max_line_length());
    }

    #[test]
    fn max_line_length_only_shows_max_line() {
        let args = Args {
            max_line_length: true,
            ..default_args()
        };
        assert!(!args.show_lines());
        assert!(!args.show_words());
        assert!(!args.show_bytes());
        assert!(args.show_max_line_length());
    }
}

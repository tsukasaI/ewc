use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ewc")]
#[command(about = "Enhanced Word Count - A modern alternative to wc")]
#[command(version)]
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

    fn show_all(&self) -> bool {
        !self.lines && !self.words && !self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shows_all() {
        let args = Args {
            files: vec![],
            lines: false,
            words: false,
            bytes: false,
        };
        assert!(args.show_lines());
        assert!(args.show_words());
        assert!(args.show_bytes());
    }

    #[test]
    fn lines_only() {
        let args = Args {
            files: vec![],
            lines: true,
            words: false,
            bytes: false,
        };
        assert!(args.show_lines());
        assert!(!args.show_words());
        assert!(!args.show_bytes());
    }

    #[test]
    fn words_only() {
        let args = Args {
            files: vec![],
            lines: false,
            words: true,
            bytes: false,
        };
        assert!(!args.show_lines());
        assert!(args.show_words());
        assert!(!args.show_bytes());
    }

    #[test]
    fn bytes_only() {
        let args = Args {
            files: vec![],
            lines: false,
            words: false,
            bytes: true,
        };
        assert!(!args.show_lines());
        assert!(!args.show_words());
        assert!(args.show_bytes());
    }

    #[test]
    fn lines_and_words() {
        let args = Args {
            files: vec![],
            lines: true,
            words: true,
            bytes: false,
        };
        assert!(args.show_lines());
        assert!(args.show_words());
        assert!(!args.show_bytes());
    }
}

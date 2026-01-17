use crate::cli::Args;
use crate::counter::Count;

pub enum OutputKind {
    File,
    Directory(usize),
}

pub fn format_number(n: usize) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_count_lines(count: &Count, args: &Args) -> Vec<String> {
    let mut lines = Vec::new();
    if args.show_lines() {
        lines.push(format!("   Lines: {:>10}", format_number(count.lines)));
    }
    if args.show_words() {
        lines.push(format!("   Words: {:>10}", format_number(count.words)));
    }
    if args.show_bytes() {
        lines.push(format!("   Bytes: {:>10}", format_number(count.bytes)));
    }
    lines
}

fn pluralize_files(count: usize) -> &'static str {
    if count == 1 {
        "file"
    } else {
        "files"
    }
}

fn format_header(name: &str, kind: OutputKind) -> String {
    match kind {
        OutputKind::File => format!("\u{1F4C4} {name}"),
        OutputKind::Directory(file_count) => {
            format!(
                "\u{1F4C1} {name} ({file_count} {})",
                pluralize_files(file_count)
            )
        }
    }
}

pub fn format_output(name: &str, count: &Count, kind: OutputKind, args: &Args) -> String {
    let mut output = vec![format_header(name, kind)];
    output.extend(format_count_lines(count, args));
    output.join("\n")
}

pub fn format_separator() -> &'static str {
    "─────────────────────────"
}

pub fn format_total_output(file_count: usize, count: &Count, args: &Args) -> String {
    let header = format!(
        "\u{1F4C1} Total ({} {})",
        file_count,
        pluralize_files(file_count)
    );
    let mut output = vec![header];
    output.extend(format_count_lines(count, args));
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(lines: bool, words: bool, bytes: bool) -> Args {
        Args {
            files: vec![],
            lines,
            words,
            bytes,
        }
    }

    #[test]
    fn format_number_without_comma() {
        assert_eq!(format_number(123), "123");
    }

    #[test]
    fn format_number_with_comma() {
        assert_eq!(format_number(1234), "1,234");
    }

    #[test]
    fn format_number_large() {
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn format_file_output_all_counts() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
        };
        let args = make_args(false, false, false);
        let output = format_output("file.txt", &count, OutputKind::File, &args);
        assert!(output.contains("file.txt"));
        assert!(output.contains("Lines:"));
        assert!(output.contains("50"));
        assert!(output.contains("Words:"));
        assert!(output.contains("200"));
        assert!(output.contains("Bytes:"));
        assert!(output.contains("1,500"));
    }

    #[test]
    fn format_file_output_lines_only() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
        };
        let args = make_args(true, false, false);
        let output = format_output("file.txt", &count, OutputKind::File, &args);
        assert!(output.contains("Lines:"));
        assert!(!output.contains("Words:"));
        assert!(!output.contains("Bytes:"));
    }

    #[test]
    fn format_separator_test() {
        let sep = format_separator();
        assert!(sep.contains("─"));
        assert_eq!(sep.chars().count(), 25);
    }

    #[test]
    fn format_total_output_all_counts() {
        let count = Count {
            lines: 80,
            words: 300,
            bytes: 2300,
        };
        let args = make_args(false, false, false);
        let output = format_total_output(2, &count, &args);
        assert!(output.contains("Total (2 files)"));
        assert!(output.contains("Lines:"));
        assert!(output.contains("80"));
        assert!(output.contains("Words:"));
        assert!(output.contains("300"));
        assert!(output.contains("Bytes:"));
        assert!(output.contains("2,300"));
    }

    #[test]
    fn format_total_pluralization() {
        let count = Count::default();
        let args = make_args(false, false, false);
        let output_single = format_total_output(1, &count, &args);
        assert!(output_single.contains("1 file)"));

        let output_plural = format_total_output(2, &count, &args);
        assert!(output_plural.contains("2 files)"));
    }

    #[test]
    fn format_total_output_lines_only() {
        let count = Count {
            lines: 80,
            words: 300,
            bytes: 2300,
        };
        let args = make_args(true, false, false);
        let output = format_total_output(2, &count, &args);
        assert!(output.contains("Lines:"));
        assert!(!output.contains("Words:"));
        assert!(!output.contains("Bytes:"));
    }

    #[test]
    fn format_directory_output_all_counts() {
        let count = Count {
            lines: 1234,
            words: 5678,
            bytes: 45000,
        };
        let args = make_args(false, false, false);
        let output = format_output("src/", &count, OutputKind::Directory(5), &args);
        assert!(output.contains("\u{1F4C1} src/ (5 files)"));
        assert!(output.contains("Lines:"));
        assert!(output.contains("1,234"));
        assert!(output.contains("Words:"));
        assert!(output.contains("5,678"));
        assert!(output.contains("Bytes:"));
        assert!(output.contains("45,000"));
    }

    #[test]
    fn format_directory_output_single_file() {
        let count = Count {
            lines: 10,
            words: 20,
            bytes: 100,
        };
        let args = make_args(false, false, false);
        let output = format_output("dir/", &count, OutputKind::Directory(1), &args);
        assert!(output.contains("\u{1F4C1} dir/ (1 file)"));
    }
}

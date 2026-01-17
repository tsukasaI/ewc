use crate::counter::Count;

pub fn format_number(n: usize) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",")
}

fn format_count_lines(
    output: &mut Vec<String>,
    count: &Count,
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
) {
    if show_lines {
        output.push(format!("   Lines: {:>10}", format_number(count.lines)));
    }
    if show_words {
        output.push(format!("   Words: {:>10}", format_number(count.words)));
    }
    if show_bytes {
        output.push(format!("   Bytes: {:>10}", format_number(count.bytes)));
    }
}

pub fn format_file_output(
    filename: &str,
    count: &Count,
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
) -> String {
    let mut output = vec![format!("\u{1F4C4} {}", filename)];
    format_count_lines(&mut output, count, show_lines, show_words, show_bytes);
    output.join("\n")
}

pub fn format_separator() -> &'static str {
    "─────────────────────────"
}

pub fn format_total_output(
    file_count: usize,
    count: &Count,
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
) -> String {
    let file_word = if file_count == 1 { "file" } else { "files" };
    let mut output = vec![format!("\u{1F4C1} Total ({} {})", file_count, file_word)];
    format_count_lines(&mut output, count, show_lines, show_words, show_bytes);
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let output = format_file_output("file.txt", &count, true, true, true);
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
        let output = format_file_output("file.txt", &count, true, false, false);
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
        let output = format_total_output(2, &count, true, true, true);
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
        let output_single = format_total_output(1, &count, true, true, true);
        assert!(output_single.contains("1 file)"));

        let output_plural = format_total_output(2, &count, true, true, true);
        assert!(output_plural.contains("2 files)"));
    }

    #[test]
    fn format_total_output_lines_only() {
        let count = Count {
            lines: 80,
            words: 300,
            bytes: 2300,
        };
        let output = format_total_output(2, &count, true, false, false);
        assert!(output.contains("Lines:"));
        assert!(!output.contains("Words:"));
        assert!(!output.contains("Bytes:"));
    }
}

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

pub fn format_file_output(
    filename: &str,
    count: &Count,
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
) -> String {
    let mut output = vec![format!("\u{1F4C4} {}", filename)];

    if show_lines {
        output.push(format!("   Lines: {:>10}", format_number(count.lines)));
    }
    if show_words {
        output.push(format!("   Words: {:>10}", format_number(count.words)));
    }
    if show_bytes {
        output.push(format!("   Bytes: {:>10}", format_number(count.bytes)));
    }

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
}

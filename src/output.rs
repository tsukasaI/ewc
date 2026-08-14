use crate::cli::Args;
use crate::counter::{Count, FileEntry};
use serde::Serialize;

pub enum OutputKind {
    File,
    Directory(usize),
}

pub fn format_number(n: u64) -> String {
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
    if args.show_max_line_length() {
        lines.push(format!(
            "Max Line: {:>10}",
            format_number(count.max_line_length)
        ));
    }
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

const FILE_ICON: &str = "\u{1F4C4} ";
const DIR_ICON: &str = "\u{1F4C1} ";

fn format_header(name: &str, kind: OutputKind, no_color: bool) -> String {
    match kind {
        OutputKind::File => {
            let icon = if no_color { "" } else { FILE_ICON };
            format!("{icon}{name}")
        }
        OutputKind::Directory(file_count) => {
            let icon = if no_color { "" } else { DIR_ICON };
            format!(
                "{icon}{name} ({file_count} {})",
                pluralize_files(file_count)
            )
        }
    }
}

pub fn format_output(name: &str, count: &Count, kind: OutputKind, args: &Args) -> String {
    let mut output = vec![format_header(name, kind, args.no_color)];
    output.extend(format_count_lines(count, args));
    output.join("\n")
}

pub fn format_separator() -> &'static str {
    "─────────────────────────"
}

fn format_compact_counts(count: &Count, args: &Args) -> String {
    let mut parts = Vec::new();
    if args.show_max_line_length() {
        parts.push(format!("max:{}", format_number(count.max_line_length)));
    }
    if args.show_lines() {
        parts.push(format!("{} lines", format_number(count.lines)));
    }
    if args.show_words() {
        parts.push(format!("{} words", format_number(count.words)));
    }
    if args.show_bytes() {
        parts.push(format!("{} bytes", format_number(count.bytes)));
    }
    parts.join(", ")
}

pub fn format_compact_output(name: &str, count: &Count, kind: OutputKind, args: &Args) -> String {
    let header = match kind {
        OutputKind::File => format!("{name}:"),
        OutputKind::Directory(file_count) => {
            format!("{name} ({file_count} {}): ", pluralize_files(file_count))
        }
    };
    format!("{header} {}", format_compact_counts(count, args))
}

pub fn format_compact_total(file_count: usize, count: &Count, args: &Args) -> String {
    format!(
        "Total ({} {}): {}",
        file_count,
        pluralize_files(file_count),
        format_compact_counts(count, args)
    )
}

fn format_single_count(count: &Count, args: &Args) -> String {
    let (value, unit) = match (args.lines, args.words, args.bytes, args.max_line_length) {
        (false, true, false, false) => (count.words, "words"),
        (false, false, true, false) => (count.bytes, "bytes"),
        (false, false, false, true) => (count.max_line_length, "max"),
        _ => (count.lines, "lines"),
    };
    format!("{} {unit}", format_number(value))
}

fn format_verbose_entry(entry: &FileEntry, args: &Args) -> String {
    let icon = if args.no_color { "" } else { FILE_ICON };
    format!(
        "{icon}{}  {}",
        entry.path.display(),
        format_single_count(&entry.count, args)
    )
}

pub fn format_verbose_output(entries: &[FileEntry], total: &Count, args: &Args) -> String {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|e| format_verbose_entry(e, args))
        .collect();

    lines.push(format_separator().to_string());

    let icon = if args.no_color { "" } else { DIR_ICON };
    let file_count = entries.len();
    lines.push(format!(
        "{icon}Total ({file_count} {})  {}",
        pluralize_files(file_count),
        format_single_count(total, args)
    ));

    lines.join("\n")
}

// JSON output structures
pub struct JsonFileResult {
    pub name: String,
    pub count: Count,
    pub is_directory: bool,
    pub file_count: Option<usize>,
}

#[derive(Serialize)]
struct JsonFile<'a> {
    file: &'a str,
    max_line_length: u64,
    lines: u64,
    words: u64,
    bytes: u64,
}

#[derive(Serialize)]
struct JsonDirectory<'a> {
    directory: &'a str,
    file_count: usize,
    max_line_length: u64,
    lines: u64,
    words: u64,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(untagged)]
enum JsonEntry<'a> {
    File(JsonFile<'a>),
    Directory(JsonDirectory<'a>),
}

#[derive(Serialize)]
struct JsonTotal {
    file_count: usize,
    max_line_length: u64,
    lines: u64,
    words: u64,
    bytes: u64,
}

#[derive(Serialize)]
struct JsonMultiple<'a> {
    files: Vec<JsonEntry<'a>>,
    total: JsonTotal,
}

fn json_entry(result: &JsonFileResult) -> JsonEntry<'_> {
    let count = &result.count;
    if result.is_directory {
        JsonEntry::Directory(JsonDirectory {
            directory: &result.name,
            file_count: result.file_count.unwrap_or(0),
            max_line_length: count.max_line_length,
            lines: count.lines,
            words: count.words,
            bytes: count.bytes,
        })
    } else {
        JsonEntry::File(JsonFile {
            file: &result.name,
            max_line_length: count.max_line_length,
            lines: count.lines,
            words: count.words,
            bytes: count.bytes,
        })
    }
}

pub fn format_json_single(result: &JsonFileResult) -> String {
    serde_json::to_string(&json_entry(result)).expect("JsonEntry serialization cannot fail")
}

pub fn format_json_multiple(results: &[JsonFileResult], total: &Count) -> String {
    let files: Vec<JsonEntry> = results.iter().map(json_entry).collect();
    let total_file_count: usize = results.iter().map(|r| r.file_count.unwrap_or(1)).sum();

    let payload = JsonMultiple {
        files,
        total: JsonTotal {
            file_count: total_file_count,
            max_line_length: total.max_line_length,
            lines: total.lines,
            words: total.words,
            bytes: total.bytes,
        },
    };

    serde_json::to_string(&payload).expect("JsonMultiple serialization cannot fail")
}

pub fn format_total_output(file_count: usize, count: &Count, args: &Args) -> String {
    let icon = if args.no_color { "" } else { DIR_ICON };
    let header = format!("{icon}Total ({file_count} {})", pluralize_files(file_count));
    let mut output = vec![header];
    output.extend(format_count_lines(count, args));
    output.join("\n")
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
            exclude: vec![],
            include: vec![],
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
            max_line_length: 80,
        };
        let args = default_args();
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
            max_line_length: 80,
        };
        let args = Args {
            lines: true,
            ..default_args()
        };
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
            max_line_length: 120,
        };
        let args = default_args();
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
        let args = default_args();
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
            max_line_length: 120,
        };
        let args = Args {
            lines: true,
            ..default_args()
        };
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
            max_line_length: 200,
        };
        let args = default_args();
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
            max_line_length: 50,
        };
        let args = default_args();
        let output = format_output("dir/", &count, OutputKind::Directory(1), &args);
        assert!(output.contains("\u{1F4C1} dir/ (1 file)"));
    }

    #[test]
    fn format_output_without_icons() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
            max_line_length: 80,
        };
        let args = Args {
            no_color: true,
            ..default_args()
        };
        let output = format_output("file.txt", &count, OutputKind::File, &args);
        assert!(!output.contains("\u{1F4C4}")); // No file icon
        assert!(output.contains("file.txt"));
    }

    #[test]
    fn format_directory_output_without_icons() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
            max_line_length: 80,
        };
        let args = Args {
            no_color: true,
            ..default_args()
        };
        let output = format_output("src/", &count, OutputKind::Directory(3), &args);
        assert!(!output.contains("\u{1F4C1}")); // No folder icon
        assert!(output.contains("src/"));
    }

    #[test]
    fn format_total_output_without_icons() {
        let count = Count {
            lines: 80,
            words: 300,
            bytes: 2300,
            max_line_length: 120,
        };
        let args = Args {
            no_color: true,
            ..default_args()
        };
        let output = format_total_output(2, &count, &args);
        assert!(!output.contains("\u{1F4C1}")); // No folder icon
        assert!(output.contains("Total (2 files)"));
    }

    #[test]
    fn format_compact_output_all_counts() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
            max_line_length: 80,
        };
        let args = Args {
            compact: true,
            ..default_args()
        };
        let output = format_compact_output("file.txt", &count, OutputKind::File, &args);
        assert!(output.contains("file.txt:"));
        assert!(output.contains("50 lines"));
        assert!(output.contains("200 words"));
        assert!(output.contains("1,500 bytes"));
        assert_eq!(output.lines().count(), 1);
    }

    #[test]
    fn format_compact_output_lines_only() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
            max_line_length: 80,
        };
        let args = Args {
            lines: true,
            compact: true,
            ..default_args()
        };
        let output = format_compact_output("file.txt", &count, OutputKind::File, &args);
        assert!(output.contains("50 lines"));
        assert!(!output.contains("words"));
        assert!(!output.contains("bytes"));
    }

    #[test]
    fn format_compact_directory() {
        let count = Count {
            lines: 150,
            words: 500,
            bytes: 3000,
            max_line_length: 100,
        };
        let args = Args {
            compact: true,
            ..default_args()
        };
        let output = format_compact_output("src/", &count, OutputKind::Directory(3), &args);
        assert!(output.contains("src/ (3 files):"));
        assert!(output.contains("150 lines"));
    }

    #[test]
    fn format_compact_total_output() {
        let count = Count {
            lines: 235,
            words: 800,
            bytes: 5000,
            max_line_length: 150,
        };
        let args = Args {
            compact: true,
            ..default_args()
        };
        let output = format_compact_total(5, &count, &args);
        assert!(output.contains("Total (5 files):"));
        assert!(output.contains("235 lines"));
    }

    #[test]
    fn format_output_max_line_length_only() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
            max_line_length: 120,
        };
        let args = Args {
            max_line_length: true,
            ..default_args()
        };
        let output = format_output("file.txt", &count, OutputKind::File, &args);
        assert!(output.contains("Max Line:"));
        assert!(output.contains("120"));
        assert!(!output.contains("Lines:"));
        assert!(!output.contains("Words:"));
        assert!(!output.contains("Bytes:"));
    }

    #[test]
    fn format_compact_with_max_line_length() {
        let count = Count {
            lines: 50,
            words: 200,
            bytes: 1500,
            max_line_length: 120,
        };
        let args = Args {
            max_line_length: true,
            compact: true,
            ..default_args()
        };
        let output = format_compact_output("file.txt", &count, OutputKind::File, &args);
        assert!(output.contains("max:120"));
        assert!(!output.contains("lines"));
    }

    #[test]
    fn json_single_escapes_all_control_characters_in_name() {
        // Backspace, form feed, and escape are all legal in POSIX filenames
        // but are not among \\, ", \n, \r, \t — the set a hand-rolled
        // escaper would need to special-case individually.
        let name = "a\u{08}b\u{0C}c\u{1B}d".to_string();
        let result = JsonFileResult {
            name,
            count: Count::default(),
            is_directory: false,
            file_count: None,
        };

        let json = format_json_single(&result);

        // Must be valid, parseable JSON with the exact original name recovered.
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["file"], "a\u{08}b\u{0C}c\u{1B}d");
        // The raw bytes must not appear unescaped in the emitted JSON text.
        assert!(!json.contains('\u{08}'));
        assert!(!json.contains('\u{0C}'));
        assert!(!json.contains('\u{1B}'));
    }

    #[test]
    fn json_single_field_order_matches_documented_example() {
        let result = JsonFileResult {
            name: "file.txt".to_string(),
            count: Count {
                lines: 50,
                words: 200,
                bytes: 1500,
                max_line_length: 120,
            },
            is_directory: false,
            file_count: None,
        };

        let json = format_json_single(&result);
        assert_eq!(
            json,
            r#"{"file":"file.txt","max_line_length":120,"lines":50,"words":200,"bytes":1500}"#
        );
    }

    #[test]
    fn json_single_directory_field_order_matches_documented_shape() {
        let result = JsonFileResult {
            name: "src".to_string(),
            count: Count {
                lines: 10,
                words: 40,
                bytes: 300,
                max_line_length: 20,
            },
            is_directory: true,
            file_count: Some(3),
        };

        let json = format_json_single(&result);
        assert_eq!(
            json,
            r#"{"directory":"src","file_count":3,"max_line_length":20,"lines":10,"words":40,"bytes":300}"#
        );
    }

    #[test]
    fn json_multiple_escapes_control_characters_and_nests_total() {
        let name = "weird\u{1B}name.txt".to_string();
        let results = vec![
            JsonFileResult {
                name: name.clone(),
                count: Count {
                    lines: 1,
                    words: 2,
                    bytes: 10,
                    max_line_length: 5,
                },
                is_directory: false,
                file_count: None,
            },
            JsonFileResult {
                name: "dir".to_string(),
                count: Count {
                    lines: 3,
                    words: 4,
                    bytes: 20,
                    max_line_length: 8,
                },
                is_directory: true,
                file_count: Some(2),
            },
        ];
        let total = Count {
            lines: 4,
            words: 6,
            bytes: 30,
            max_line_length: 8,
        };

        let json = format_json_multiple(&results, &total);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["files"][0]["file"], "weird\u{1B}name.txt");
        assert!(!json.contains('\u{1B}'));
        assert_eq!(parsed["files"][1]["directory"], "dir");
        assert_eq!(parsed["total"]["file_count"], 3); // 1 file + 2 in the directory
        assert_eq!(parsed["total"]["lines"], 4);
    }
}

use crate::cli::Args;
use crate::counter::{Count, FileEntry};
use serde::Serialize;
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub enum OutputKind {
    File,
    Directory(usize),
}

/// Replaces C0 and C1 control characters and DEL with the Unicode
/// replacement character, so a filename containing an embedded terminal
/// escape sequence can't manipulate the terminal when printed. Callers pass
/// whether the destination stream is actually a terminal: a piped or
/// redirected stream doesn't interpret escape codes, so there's nothing to
/// guard there, and JSON output escapes only C0 control characters via
/// serde (a JSON-validity concern, not a terminal-safety one) regardless
/// of this function.
pub fn sanitize_for_display(name: &str, is_terminal: bool) -> Cow<'_, str> {
    // char::is_control() is exactly Unicode Cc: C0 (0x00-0x1F), DEL (0x7F),
    // and C1 (0x80-0x9F). C1 matters here as much as C0 does -- U+009B and
    // U+009D are the single-byte forms of CSI and OSC, and some terminals
    // (including xterm's UTF-8 C1 handling) execute them the same as the
    // two-byte ESC-prefixed sequences.
    if !is_terminal || name.chars().all(|c| !c.is_control()) {
        return Cow::Borrowed(name);
    }
    Cow::Owned(
        name.chars()
            .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
            .collect(),
    )
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

pub fn icon(no_color: bool, glyph: &'static str) -> &'static str {
    if no_color {
        ""
    } else {
        glyph
    }
}

fn format_header(name: &str, kind: OutputKind, no_color: bool, is_terminal: bool) -> String {
    let name = sanitize_for_display(name, is_terminal);
    match kind {
        OutputKind::File => format!("{}{name}", icon(no_color, FILE_ICON)),
        OutputKind::Directory(file_count) => {
            format!(
                "{}{name} ({file_count} {})",
                icon(no_color, DIR_ICON),
                pluralize_files(file_count)
            )
        }
    }
}

pub fn format_output(
    name: &str,
    count: &Count,
    kind: OutputKind,
    args: &Args,
    is_terminal: bool,
) -> String {
    let mut output = vec![format_header(name, kind, args.no_color, is_terminal)];
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

pub fn format_compact_output(
    name: &str,
    count: &Count,
    kind: OutputKind,
    args: &Args,
    is_terminal: bool,
) -> String {
    let name = sanitize_for_display(name, is_terminal);
    let header = match kind {
        OutputKind::File => format!("{name}:"),
        OutputKind::Directory(file_count) => {
            format!("{name} ({file_count} {}):", pluralize_files(file_count))
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

/// First-enabled-metric lookup for verbose mode's one-metric-per-line
/// display. Lines must stay before words/bytes (so the flagless default
/// shows lines) and max must stay last (so `-l -L` shows lines, not max).
fn format_single_count(count: &Count, args: &Args) -> String {
    let (value, unit) = if args.show_lines() {
        (count.lines, "lines")
    } else if args.show_words() {
        (count.words, "words")
    } else if args.show_bytes() {
        (count.bytes, "bytes")
    } else {
        (count.max_line_length, "max")
    };
    format!("{} {unit}", format_number(value))
}

fn format_verbose_entry(entry: &FileEntry, args: &Args, is_terminal: bool) -> String {
    let path_str = entry.path.display().to_string();
    format!(
        "{}{}  {}",
        icon(args.no_color, FILE_ICON),
        sanitize_for_display(&path_str, is_terminal),
        format_single_count(&entry.count, args)
    )
}

pub fn format_verbose_output(
    entries: &[FileEntry],
    total: &Count,
    args: &Args,
    is_terminal: bool,
) -> String {
    let mut lines: Vec<String> = entries
        .iter()
        .map(|e| format_verbose_entry(e, args, is_terminal))
        .collect();

    lines.push(format_separator().to_string());

    let file_count = entries.len();
    lines.push(format!(
        "{}Total ({file_count} {})  {}",
        icon(args.no_color, DIR_ICON),
        pluralize_files(file_count),
        format_single_count(total, args)
    ));

    lines.join("\n")
}

// JSON output structures
pub struct JsonFileResult {
    pub name: String,
    pub count: Count,
    pub kind: OutputKind,
    /// Directory entries or sub-paths that couldn't be counted (permission
    /// denied, vanished mid-walk, etc.). A directory's totals are complete
    /// only when this is zero (#87).
    pub skipped_count: usize,
}

fn is_zero(n: &usize) -> bool {
    *n == 0
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
    #[serde(skip_serializing_if = "is_zero")]
    skipped_count: usize,
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
    #[serde(skip_serializing_if = "is_zero")]
    skipped_count: usize,
}

#[derive(Serialize)]
struct JsonMultiple<'a> {
    files: Vec<JsonEntry<'a>>,
    total: JsonTotal,
}

fn json_entry(result: &JsonFileResult) -> JsonEntry<'_> {
    let count = &result.count;
    match result.kind {
        OutputKind::Directory(file_count) => JsonEntry::Directory(JsonDirectory {
            directory: &result.name,
            file_count,
            max_line_length: count.max_line_length,
            lines: count.lines,
            words: count.words,
            bytes: count.bytes,
            skipped_count: result.skipped_count,
        }),
        OutputKind::File => JsonEntry::File(JsonFile {
            file: &result.name,
            max_line_length: count.max_line_length,
            lines: count.lines,
            words: count.words,
            bytes: count.bytes,
        }),
    }
}

pub fn format_json_single(result: &JsonFileResult) -> String {
    serde_json::to_string(&json_entry(result)).expect("JsonEntry serialization cannot fail")
}

pub fn format_json_multiple(results: &[JsonFileResult], total: &Count) -> String {
    let files: Vec<JsonEntry> = results.iter().map(json_entry).collect();
    let total_file_count: usize = results
        .iter()
        .map(|r| match r.kind {
            OutputKind::File => 1,
            OutputKind::Directory(file_count) => file_count,
        })
        .sum();
    let total_skipped_count: usize = results.iter().map(|r| r.skipped_count).sum();

    let payload = JsonMultiple {
        files,
        total: JsonTotal {
            file_count: total_file_count,
            skipped_count: total_skipped_count,
            max_line_length: total.max_line_length,
            lines: total.lines,
            words: total.words,
            bytes: total.bytes,
        },
    };

    serde_json::to_string(&payload).expect("JsonMultiple serialization cannot fail")
}

pub fn format_total_output(file_count: usize, count: &Count, args: &Args) -> String {
    let header = format!(
        "{}Total ({file_count} {})",
        icon(args.no_color, DIR_ICON),
        pluralize_files(file_count)
    );
    let mut output = vec![header];
    output.extend(format_count_lines(count, args));
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::default_args;

    fn single_entry(count: Count) -> Vec<FileEntry> {
        vec![FileEntry {
            path: "file.txt".into(),
            count,
        }]
    }

    #[test]
    fn sanitize_for_display_replaces_control_chars_when_terminal() {
        // ESC (used by ANSI escape sequences) and a raw newline, both of
        // which could manipulate or forge lines in a terminal (#81).
        let name = "evil\x1b[31mred\x1b[0m\nname.txt";
        let sanitized = sanitize_for_display(name, true);
        assert!(!sanitized.contains('\x1b'));
        assert!(!sanitized.contains('\n'));
        assert!(sanitized.contains('\u{FFFD}'));
    }

    #[test]
    fn sanitize_for_display_passes_through_when_not_a_terminal() {
        // Piped/redirected output doesn't interpret escape codes, and JSON
        // output escapes control characters separately (via serde) -- this
        // sanitization is purely a terminal-display safeguard.
        let name = "evil\x1b[31mname.txt";
        assert_eq!(sanitize_for_display(name, false), name);
    }

    #[test]
    fn sanitize_for_display_replaces_c1_control_chars() {
        // U+009B and U+009D are the single-byte forms of CSI and OSC; some
        // terminals execute them the same as the two-byte ESC-prefixed
        // sequences, so they need the same treatment as C0/DEL.
        let name = "\u{9b}[31mred\u{9d}0;evil\u{9c}.txt";
        let sanitized = sanitize_for_display(name, true);
        assert!(!sanitized.contains('\u{9b}'));
        assert!(!sanitized.contains('\u{9d}'));
        assert!(!sanitized.contains('\u{9c}'));
    }

    #[test]
    fn sanitize_for_display_passes_through_clean_names_unchanged() {
        let name = "ordinary_file.txt";
        assert_eq!(sanitize_for_display(name, true), name);
    }

    #[test]
    fn format_output_sanitizes_the_name_when_is_terminal() {
        let count = Count::default();
        let args = default_args();
        let output = format_output("evil\x1bname.txt", &count, OutputKind::File, &args, true);
        assert!(!output.contains('\x1b'));

        let output = format_output("evil\x1bname.txt", &count, OutputKind::File, &args, false);
        assert!(output.contains('\x1b'));
    }

    #[test]
    fn format_compact_output_sanitizes_the_name_when_is_terminal() {
        let count = Count::default();
        let args = Args {
            compact: true,
            ..default_args()
        };
        let output =
            format_compact_output("evil\x1bname.txt", &count, OutputKind::File, &args, true);
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn format_verbose_output_sanitizes_the_path_when_is_terminal() {
        let entries = vec![FileEntry {
            path: "evil\x1bname.txt".into(),
            count: Count::default(),
        }];
        let args = default_args();
        let output = format_verbose_output(&entries, &Count::default(), &args, true);
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn verbose_single_metric_uses_args_show_helpers() {
        // Regression test for #53.
        let count = Count {
            lines: 1,
            words: 2,
            bytes: 3,
            max_line_length: 4,
        };
        let entries = single_entry(count);

        // -w -c (no -l): must show the first requested metric (words), not
        // fall back to lines.
        let args = Args {
            words: true,
            bytes: true,
            ..default_args()
        };
        let output = format_verbose_output(&entries, &count, &args, false);
        assert!(output.contains("2 words"));
        assert!(!output.contains("1 lines"));

        // -L alone: must show max, not fall back to lines.
        let args = Args {
            max_line_length: true,
            ..default_args()
        };
        let output = format_verbose_output(&entries, &count, &args, false);
        assert!(output.contains("4 max"));
        assert!(!output.contains("1 lines"));

        // -l -L: lines wins even though max is also requested.
        let args = Args {
            lines: true,
            max_line_length: true,
            ..default_args()
        };
        let output = format_verbose_output(&entries, &count, &args, false);
        assert!(output.contains("1 lines"));
        assert!(!output.contains("4 max"));
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
        let output = format_output("file.txt", &count, OutputKind::File, &args, false);
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
        let output = format_output("file.txt", &count, OutputKind::File, &args, false);
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
        let output = format_output("src/", &count, OutputKind::Directory(5), &args, false);
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
        let output = format_output("dir/", &count, OutputKind::Directory(1), &args, false);
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
        let output = format_output("file.txt", &count, OutputKind::File, &args, false);
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
        let output = format_output("src/", &count, OutputKind::Directory(3), &args, false);
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
        let output = format_compact_output("file.txt", &count, OutputKind::File, &args, false);
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
        let output = format_compact_output("file.txt", &count, OutputKind::File, &args, false);
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
        let output = format_compact_output("src/", &count, OutputKind::Directory(3), &args, false);
        assert!(output.contains("150 lines"));
        // Exact match on the separator, not just a substring: this pins the
        // single space between "files):" and the counts, matching
        // format_compact_total's spacing for the identical shape (#51).
        assert!(output.starts_with("src/ (3 files): 150 lines"));
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
        // Exact match, not a substring: pins the single-space spacing this
        // shares with format_compact_output's Directory arm (#51).
        assert!(output.starts_with("Total (5 files): 235 lines"));
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
        let output = format_output("file.txt", &count, OutputKind::File, &args, false);
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
        let output = format_compact_output("file.txt", &count, OutputKind::File, &args, false);
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
            kind: OutputKind::File,
            skipped_count: 0,
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
            kind: OutputKind::File,
            skipped_count: 0,
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
            kind: OutputKind::Directory(3),
            skipped_count: 0,
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
                kind: OutputKind::File,
                skipped_count: 0,
            },
            JsonFileResult {
                name: "dir".to_string(),
                count: Count {
                    lines: 3,
                    words: 4,
                    bytes: 20,
                    max_line_length: 8,
                },
                kind: OutputKind::Directory(2),
                skipped_count: 1,
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
        assert_eq!(parsed["files"][1]["skipped_count"], 1);
        assert_eq!(parsed["total"]["file_count"], 3); // 1 file + 2 in the directory
        assert_eq!(parsed["total"]["lines"], 4);
        assert_eq!(parsed["total"]["skipped_count"], 1);
    }

    #[test]
    fn json_skipped_count_omitted_when_zero() {
        // The common (no failures) case must not grow a new key, so
        // existing consumers parsing a fixed schema aren't surprised (#87).
        let result = JsonFileResult {
            name: "src".to_string(),
            count: Count::default(),
            kind: OutputKind::Directory(3),
            skipped_count: 0,
        };
        let json = format_json_single(&result);
        assert!(!json.contains("skipped_count"));
    }
}

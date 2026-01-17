use std::io::Write;
use std::process::Command;

fn create_test_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "{content}").unwrap();
    file
}

struct CommandResult {
    stdout: String,
    stderr: String,
    success: bool,
}

fn run_ewc(args: &[&str]) -> CommandResult {
    let output = Command::new("./target/debug/ewc")
        .args(args)
        .output()
        .expect("failed to run ewc");
    CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    }
}

#[test]
fn single_file_no_total() {
    let file = create_test_file("hello world\n");
    let result = run_ewc(&[file.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("Lines:"));
    assert!(!result.stdout.contains("Total"));
}

#[test]
fn multiple_files_shows_total() {
    let file1 = create_test_file("hello world\n");
    let file2 = create_test_file("foo bar baz\n");
    let result = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("Total (2 files)"));
    assert!(result.stdout.contains("─────────────────────────"));
}

#[test]
fn multiple_files_correct_aggregation() {
    let file1 = create_test_file("line one\nline two\n");
    let file2 = create_test_file("line three\n");
    let result = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("Total (2 files)"));
    // Total should be 3 lines
    let total_section: String = result
        .stdout
        .lines()
        .skip_while(|l| !l.contains("Total"))
        .collect();
    assert!(total_section.contains("3"));
}

#[test]
fn error_with_valid_files_shows_partial_total() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let result = run_ewc(&[
        file1.path().to_str().unwrap(),
        "nonexistent.txt",
        file2.path().to_str().unwrap(),
    ]);

    assert!(!result.success); // Should fail due to error
    assert!(result.stderr.contains("nonexistent.txt"));
    assert!(result.stdout.contains("Total (2 files)")); // Only 2 successful files
}

#[test]
fn single_error_among_multiple_no_total_if_one_success() {
    let file1 = create_test_file("hello\n");
    let result = run_ewc(&[file1.path().to_str().unwrap(), "nonexistent.txt"]);

    assert!(!result.success);
    assert!(result.stderr.contains("nonexistent.txt"));
    assert!(!result.stdout.contains("Total")); // Only 1 successful file, no total
}

#[test]
fn flags_respected_in_total() {
    let file1 = create_test_file("hello world\n");
    let file2 = create_test_file("foo bar\n");
    let result = run_ewc(&[
        "-l",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("Total (2 files)"));
    assert!(result.stdout.contains("Lines:"));
    assert!(!result.stdout.contains("Words:"));
    assert!(!result.stdout.contains("Bytes:"));
}

#[test]
fn words_only_flag_in_total() {
    let file1 = create_test_file("hello world\n");
    let file2 = create_test_file("foo\n");
    let result = run_ewc(&[
        "-w",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("Total (2 files)"));
    assert!(!result.stdout.contains("Lines:"));
    assert!(result.stdout.contains("Words:"));
    assert!(!result.stdout.contains("Bytes:"));
}

#[test]
fn three_files_shows_correct_count() {
    let file1 = create_test_file("a\n");
    let file2 = create_test_file("b\n");
    let file3 = create_test_file("c\n");
    let result = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
        file3.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("Total (3 files)"));
}

#[test]
fn blank_lines_between_files() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let result = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    // Should have blank line between file outputs
    assert!(result.stdout.contains("\n\n"));
}

fn create_test_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file1.txt"), "hello world\n").unwrap();
    std::fs::write(dir.path().join("file2.txt"), "foo bar baz\n").unwrap();
    dir
}

#[test]
fn directory_shows_summary() {
    let dir = create_test_dir();
    let result = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("📁"));
    assert!(result.stdout.contains("(2 files)"));
    assert!(result.stdout.contains("Lines:"));
}

#[test]
fn directory_excludes_hidden_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("visible.txt"), "visible\n").unwrap();
    std::fs::write(dir.path().join(".hidden"), "hidden\n").unwrap();

    let result = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("(1 file)"));
}

#[test]
fn directory_with_nested_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();
    let subdir = dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("nested.txt"), "nested\n").unwrap();

    let result = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("(2 files)"));
}

// Phase 4: --no-color tests
#[test]
fn no_color_flag_removes_icons() {
    let file = create_test_file("hello world\n");
    let result = run_ewc(&["--no-color", file.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(!result.stdout.contains("📄"));
    assert!(!result.stdout.contains("📁"));
    assert!(result.stdout.contains("Lines:"));
}

#[test]
fn no_color_flag_directory() {
    let dir = create_test_dir();
    let result = run_ewc(&["--no-color", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(!result.stdout.contains("📁"));
    assert!(result.stdout.contains("(2 files)"));
}

// Phase 4: -a/--all tests
#[test]
fn all_flag_includes_hidden_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("visible.txt"), "visible\n").unwrap();
    std::fs::write(dir.path().join(".hidden"), "hidden\n").unwrap();

    let result = run_ewc(&["-a", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("(2 files)")); // Both files counted
}

#[test]
fn all_flag_includes_hidden_directories() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("visible.txt"), "visible\n").unwrap();
    let hidden_dir = dir.path().join(".hidden_dir");
    std::fs::create_dir(&hidden_dir).unwrap();
    std::fs::write(hidden_dir.join("nested.txt"), "nested\n").unwrap();

    let result = run_ewc(&["-a", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("(2 files)")); // Both files counted
}

#[test]
fn without_all_flag_excludes_hidden() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("visible.txt"), "visible\n").unwrap();
    std::fs::write(dir.path().join(".hidden"), "hidden\n").unwrap();

    let result = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("(1 file)")); // Only visible file counted
}

// Phase 4: -C/--compact tests
#[test]
fn compact_flag_single_file() {
    let file = create_test_file("hello world\n");
    let result = run_ewc(&["-C", file.path().to_str().unwrap()]);

    assert!(result.success);
    // Should be single line
    assert_eq!(result.stdout.trim().lines().count(), 1);
    assert!(result.stdout.contains("lines"));
    assert!(result.stdout.contains("words"));
    assert!(result.stdout.contains("bytes"));
}

#[test]
fn compact_flag_multiple_files() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let result = run_ewc(&[
        "-C",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("Total"));
}

#[test]
fn compact_with_lines_flag() {
    let file = create_test_file("hello world\n");
    let result = run_ewc(&["-C", "-l", file.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("lines"));
    assert!(!result.stdout.contains("words"));
    assert!(!result.stdout.contains("bytes"));
}

// Phase 4: -v/--verbose tests
#[test]
fn verbose_flag_shows_file_list() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file1.txt"), "hello\n").unwrap();
    std::fs::write(dir.path().join("file2.txt"), "world\n").unwrap();

    let result = run_ewc(&["-v", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("file1.txt"));
    assert!(result.stdout.contains("file2.txt"));
    assert!(result.stdout.contains("Total"));
}

#[test]
fn verbose_flag_with_nested_directories() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();
    let subdir = dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("nested.txt"), "nested\n").unwrap();

    let result = run_ewc(&["-v", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("root.txt"));
    assert!(result.stdout.contains("nested.txt"));
}

#[test]
fn verbose_flag_not_applicable_to_single_file() {
    let file = create_test_file("hello\n");
    let result = run_ewc(&["-v", file.path().to_str().unwrap()]);

    assert!(result.success);
    // Should behave same as without -v for single file
    assert!(result.stdout.contains("Lines:"));
}

#[test]
fn verbose_with_no_color() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file1.txt"), "hello\n").unwrap();

    let result = run_ewc(&["-v", "--no-color", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(!result.stdout.contains("📄"));
    assert!(result.stdout.contains("file1.txt"));
}

// Phase 4: --json tests
#[test]
fn json_flag_single_file() {
    let file = create_test_file("hello world\n");
    let result = run_ewc(&["--json", file.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("{"));
    assert!(result.stdout.contains("\"lines\""));
    assert!(result.stdout.contains("\"words\""));
    assert!(result.stdout.contains("\"bytes\""));
}

#[test]
fn json_flag_multiple_files() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let result = run_ewc(&[
        "--json",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(result.success);
    assert!(result.stdout.contains("\"files\""));
    assert!(result.stdout.contains("\"total\""));
}

#[test]
fn json_flag_directory() {
    let dir = create_test_dir();
    let result = run_ewc(&["--json", dir.path().to_str().unwrap()]);

    assert!(result.success);
    assert!(result.stdout.contains("\"directory\""));
    assert!(result.stdout.contains("\"file_count\""));
}

#[test]
fn json_output_is_valid() {
    let file = create_test_file("hello\n");
    let result = run_ewc(&["--json", file.path().to_str().unwrap()]);

    assert!(result.success);
    // Check it's valid JSON by parsing manually - starts with { ends with }
    let trimmed = result.stdout.trim();
    assert!(trimmed.starts_with('{'));
    assert!(trimmed.ends_with('}'));
}

// Phase 5: stdin support tests
fn run_ewc_with_stdin(args: &[&str], stdin_content: &str) -> CommandResult {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("./target/debug/ewc")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ewc");

    // Write to stdin and drop handle to send EOF
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_content.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    }
}

#[test]
fn stdin_input_basic() {
    let result = run_ewc_with_stdin(&[], "hello world\n");

    assert!(result.success);
    assert!(result.stdout.contains("<stdin>"));
    assert!(result.stdout.contains("Lines:"));
}

#[test]
fn stdin_input_no_color() {
    let result = run_ewc_with_stdin(&["--no-color"], "hello\n");

    assert!(result.success);
    assert!(result.stdout.contains("<stdin>"));
    assert!(!result.stdout.contains("📄"));
}

#[test]
fn stdin_input_json() {
    let result = run_ewc_with_stdin(&["--json"], "hello world\n");

    assert!(result.success);
    assert!(result.stdout.contains("\"file\":\"<stdin>\""));
    assert!(result.stdout.contains("\"lines\":1"));
    assert!(result.stdout.contains("\"words\":2"));
}

#[test]
fn stdin_input_compact() {
    let result = run_ewc_with_stdin(&["-C"], "hello world\n");

    assert!(result.success);
    assert!(result.stdout.contains("<stdin>:"));
    assert_eq!(result.stdout.trim().lines().count(), 1);
}

#[test]
fn stdin_input_lines_only() {
    let result = run_ewc_with_stdin(&["-l"], "line1\nline2\n");

    assert!(result.success);
    assert!(result.stdout.contains("Lines:"));
    assert!(!result.stdout.contains("Words:"));
    assert!(!result.stdout.contains("Bytes:"));
}

// Phase 5: Enhanced error handling tests
#[test]
fn error_message_file_not_found() {
    let result = run_ewc(&["definitely_not_a_real_file.txt"]);

    assert!(!result.success);
    assert!(result.stderr.contains("⚠️"));
    assert!(result.stderr.contains("definitely_not_a_real_file.txt"));
}

#[test]
fn single_file_not_found_exits_with_error() {
    let result = run_ewc(&["nonexistent_file.txt"]);

    assert!(!result.success);
    assert!(result.stderr.contains("⚠️"));
    assert!(!result.stdout.contains("Lines:"));
}

#[test]
fn json_mode_continues_after_error() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let result = run_ewc(&[
        "--json",
        file1.path().to_str().unwrap(),
        "nonexistent.txt",
        file2.path().to_str().unwrap(),
    ]);

    assert!(!result.success);
    assert!(result.stdout.contains("\"files\""));
    assert!(result.stdout.contains("\"total\""));
}

#[test]
fn directory_and_nonexistent_file() {
    let dir = create_test_dir();
    let result = run_ewc(&[dir.path().to_str().unwrap(), "nonexistent.txt"]);

    assert!(!result.success);
    assert!(result.stderr.contains("nonexistent.txt"));
    assert!(result.stdout.contains("📁"));
}

use std::io::Write;
use std::process::Command;

fn create_test_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "{}", content).unwrap();
    file
}

fn run_ewc(args: &[&str]) -> (String, String, bool) {
    let output = Command::new("./target/debug/ewc")
        .args(args)
        .output()
        .expect("failed to run ewc");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
fn single_file_no_total() {
    let file = create_test_file("hello world\n");
    let (stdout, _, success) = run_ewc(&[file.path().to_str().unwrap()]);

    assert!(success);
    assert!(stdout.contains("Lines:"));
    assert!(!stdout.contains("Total"));
}

#[test]
fn multiple_files_shows_total() {
    let file1 = create_test_file("hello world\n");
    let file2 = create_test_file("foo bar baz\n");
    let (stdout, _, success) = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(success);
    assert!(stdout.contains("Total (2 files)"));
    assert!(stdout.contains("─────────────────────────"));
}

#[test]
fn multiple_files_correct_aggregation() {
    let file1 = create_test_file("line one\nline two\n");
    let file2 = create_test_file("line three\n");
    let (stdout, _, success) = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(success);
    assert!(stdout.contains("Total (2 files)"));
    // Total should be 3 lines
    let total_section: String = stdout
        .lines()
        .skip_while(|l| !l.contains("Total"))
        .collect();
    assert!(total_section.contains("3"));
}

#[test]
fn error_with_valid_files_shows_partial_total() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let (stdout, stderr, success) = run_ewc(&[
        file1.path().to_str().unwrap(),
        "nonexistent.txt",
        file2.path().to_str().unwrap(),
    ]);

    assert!(!success); // Should fail due to error
    assert!(stderr.contains("nonexistent.txt"));
    assert!(stdout.contains("Total (2 files)")); // Only 2 successful files
}

#[test]
fn single_error_among_multiple_no_total_if_one_success() {
    let file1 = create_test_file("hello\n");
    let (stdout, stderr, success) = run_ewc(&[file1.path().to_str().unwrap(), "nonexistent.txt"]);

    assert!(!success);
    assert!(stderr.contains("nonexistent.txt"));
    assert!(!stdout.contains("Total")); // Only 1 successful file, no total
}

#[test]
fn flags_respected_in_total() {
    let file1 = create_test_file("hello world\n");
    let file2 = create_test_file("foo bar\n");
    let (stdout, _, success) = run_ewc(&[
        "-l",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(success);
    assert!(stdout.contains("Total (2 files)"));
    assert!(stdout.contains("Lines:"));
    assert!(!stdout.contains("Words:"));
    assert!(!stdout.contains("Bytes:"));
}

#[test]
fn words_only_flag_in_total() {
    let file1 = create_test_file("hello world\n");
    let file2 = create_test_file("foo\n");
    let (stdout, _, success) = run_ewc(&[
        "-w",
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(success);
    assert!(stdout.contains("Total (2 files)"));
    assert!(!stdout.contains("Lines:"));
    assert!(stdout.contains("Words:"));
    assert!(!stdout.contains("Bytes:"));
}

#[test]
fn three_files_shows_correct_count() {
    let file1 = create_test_file("a\n");
    let file2 = create_test_file("b\n");
    let file3 = create_test_file("c\n");
    let (stdout, _, success) = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
        file3.path().to_str().unwrap(),
    ]);

    assert!(success);
    assert!(stdout.contains("Total (3 files)"));
}

#[test]
fn blank_lines_between_files() {
    let file1 = create_test_file("hello\n");
    let file2 = create_test_file("world\n");
    let (stdout, _, success) = run_ewc(&[
        file1.path().to_str().unwrap(),
        file2.path().to_str().unwrap(),
    ]);

    assert!(success);
    // Should have blank line between file outputs
    assert!(stdout.contains("\n\n"));
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
    let (stdout, _, success) = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(success);
    assert!(stdout.contains("📁"));
    assert!(stdout.contains("(2 files)"));
    assert!(stdout.contains("Lines:"));
}

#[test]
fn directory_excludes_hidden_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("visible.txt"), "visible\n").unwrap();
    std::fs::write(dir.path().join(".hidden"), "hidden\n").unwrap();

    let (stdout, _, success) = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(success);
    assert!(stdout.contains("(1 file)"));
}

#[test]
fn directory_with_nested_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();
    let subdir = dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("nested.txt"), "nested\n").unwrap();

    let (stdout, _, success) = run_ewc(&[dir.path().to_str().unwrap()]);

    assert!(success);
    assert!(stdout.contains("(2 files)"));
}

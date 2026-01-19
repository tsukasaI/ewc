use std::fs;
use std::io::{self, Read};
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileEntry {
    pub path: PathBuf,
    pub count: Count,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Count {
    pub lines: usize,
    pub words: usize,
    pub bytes: usize,
    pub max_line_length: usize,
}

impl Count {
    pub fn from_content(content: &str) -> Self {
        Self {
            lines: content.lines().count(),
            words: content.split_whitespace().count(),
            bytes: content.len(),
            max_line_length: content.lines().map(|l| l.len()).max().unwrap_or(0),
        }
    }
}

impl Add for Count {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            lines: self.lines + other.lines,
            words: self.words + other.words,
            bytes: self.bytes + other.bytes,
            max_line_length: self.max_line_length.max(other.max_line_length),
        }
    }
}

impl AddAssign for Count {
    fn add_assign(&mut self, other: Self) {
        self.lines += other.lines;
        self.words += other.words;
        self.bytes += other.bytes;
        self.max_line_length = self.max_line_length.max(other.max_line_length);
    }
}

pub fn count_file(path: &Path) -> io::Result<Count> {
    let content = fs::read_to_string(path)?;
    Ok(Count::from_content(&content))
}

pub fn count_from_reader<R: Read>(mut reader: R) -> io::Result<Count> {
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    Ok(Count::from_content(&content))
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

fn walk_directory(
    path: &Path,
    include_hidden: bool,
) -> impl Iterator<Item = walkdir::DirEntry> + '_ {
    WalkDir::new(path)
        .into_iter()
        .filter_entry(move |e| e.depth() == 0 || include_hidden || !is_hidden(e))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
}

pub fn count_directory(path: &Path, include_hidden: bool) -> io::Result<(Count, usize)> {
    let (entries, total) = count_directory_detailed(path, include_hidden)?;
    Ok((total, entries.len()))
}

pub fn count_directory_detailed(
    path: &Path,
    include_hidden: bool,
) -> io::Result<(Vec<FileEntry>, Count)> {
    let mut entries: Vec<FileEntry> = walk_directory(path, include_hidden)
        .filter_map(|entry| {
            count_file(entry.path()).ok().map(|count| FileEntry {
                path: entry.path().to_path_buf(),
                count,
            })
        })
        .collect();

    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let total = entries
        .iter()
        .fold(Count::default(), |acc, e| acc + e.count.clone());
    Ok((entries, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_empty_string() {
        let count = Count::from_content("");
        assert_eq!(count.lines, 0);
        assert_eq!(count.words, 0);
        assert_eq!(count.bytes, 0);
        assert_eq!(count.max_line_length, 0);
    }

    #[test]
    fn count_single_line() {
        let count = Count::from_content("hello");
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 1);
        assert_eq!(count.bytes, 5);
        assert_eq!(count.max_line_length, 5);
    }

    #[test]
    fn count_multiple_lines() {
        let count = Count::from_content("hello\nworld");
        assert_eq!(count.lines, 2);
        assert_eq!(count.words, 2);
        assert_eq!(count.bytes, 11);
        assert_eq!(count.max_line_length, 5);
    }

    #[test]
    fn count_multiple_words() {
        let count = Count::from_content("hello world");
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
        assert_eq!(count.bytes, 11);
    }

    #[test]
    fn count_multibyte_characters() {
        // "あ" is 3 bytes in UTF-8
        let count = Count::from_content("あ");
        assert_eq!(count.bytes, 3);
    }

    #[test]
    fn count_from_content_combined() {
        let count = Count::from_content("hello world\nfoo bar");
        assert_eq!(count.lines, 2);
        assert_eq!(count.words, 4);
        assert_eq!(count.bytes, 19);
    }

    #[test]
    fn count_file_success() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "hello world").unwrap();
        writeln!(file, "foo bar").unwrap();

        let result = count_file(file.path());
        assert!(result.is_ok());
        let count = result.unwrap();
        assert_eq!(count.lines, 2);
        assert_eq!(count.words, 4);
    }

    #[test]
    fn count_file_not_found() {
        let result = count_file(Path::new("nonexistent_file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn count_default() {
        let count = Count::default();
        assert_eq!(count.lines, 0);
        assert_eq!(count.words, 0);
        assert_eq!(count.bytes, 0);
        assert_eq!(count.max_line_length, 0);
    }

    #[test]
    fn count_max_line_length_varies() {
        let count = Count::from_content("short\nlonger line here\nmed");
        assert_eq!(count.max_line_length, 16); // "longer line here"
    }

    #[test]
    fn count_add() {
        let count1 = Count {
            lines: 10,
            words: 50,
            bytes: 200,
            max_line_length: 80,
        };
        let count2 = Count {
            lines: 5,
            words: 25,
            bytes: 100,
            max_line_length: 120,
        };
        let total = count1 + count2;
        assert_eq!(total.lines, 15);
        assert_eq!(total.words, 75);
        assert_eq!(total.bytes, 300);
        assert_eq!(total.max_line_length, 120); // Takes max of the two
    }

    #[test]
    fn count_directory_with_single_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "hello world").unwrap();

        let result = count_directory(dir.path(), false);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 1);
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
    }

    #[test]
    fn count_directory_with_multiple_files() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        let file1 = dir.path().join("file1.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "hello").unwrap();

        let file2 = dir.path().join("file2.txt");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "world").unwrap();

        let result = count_directory(dir.path(), false);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 2);
        assert_eq!(count.lines, 2);
        assert_eq!(count.words, 2);
    }

    #[test]
    fn count_directory_recursive() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Root file
        let file1 = dir.path().join("root.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "root").unwrap();

        // Nested directory with file
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let file2 = subdir.join("nested.txt");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "nested file").unwrap();

        let result = count_directory(dir.path(), false);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 2);
        assert_eq!(count.lines, 2);
        assert_eq!(count.words, 3); // "root" + "nested file"
    }

    #[test]
    fn count_directory_excludes_hidden_files() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Visible file
        let file1 = dir.path().join("visible.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "visible").unwrap();

        // Hidden file (should be excluded)
        let file2 = dir.path().join(".hidden");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "hidden").unwrap();

        let result = count_directory(dir.path(), false);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 1); // Only visible file
        assert_eq!(count.words, 1); // Only "visible"
    }

    #[test]
    fn count_directory_excludes_hidden_directories() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Visible file
        let file1 = dir.path().join("visible.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "visible").unwrap();

        // Hidden directory with file (should be excluded)
        let hidden_dir = dir.path().join(".hidden_dir");
        std::fs::create_dir(&hidden_dir).unwrap();
        let file2 = hidden_dir.join("nested.txt");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "nested in hidden").unwrap();

        let result = count_directory(dir.path(), false);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 1); // Only visible file
        assert_eq!(count.words, 1); // Only "visible"
    }

    #[test]
    fn count_directory_includes_hidden_files_when_all() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Visible file
        let file1 = dir.path().join("visible.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "visible").unwrap();

        // Hidden file (should be included with include_hidden=true)
        let file2 = dir.path().join(".hidden");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "hidden").unwrap();

        let result = count_directory(dir.path(), true);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 2); // Both files
        assert_eq!(count.words, 2); // "visible" + "hidden"
    }

    #[test]
    fn count_directory_includes_hidden_directories_when_all() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        // Visible file
        let file1 = dir.path().join("visible.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "visible").unwrap();

        // Hidden directory with file (should be included with include_hidden=true)
        let hidden_dir = dir.path().join(".hidden_dir");
        std::fs::create_dir(&hidden_dir).unwrap();
        let file2 = hidden_dir.join("nested.txt");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "nested in hidden").unwrap();

        let result = count_directory(dir.path(), true);
        assert!(result.is_ok());
        let (count, file_count) = result.unwrap();
        assert_eq!(file_count, 2); // Both files
        assert_eq!(count.words, 4); // "visible" + "nested in hidden"
    }

    #[test]
    fn count_directory_detailed_returns_file_entries() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();

        let file1 = dir.path().join("file1.txt");
        let mut f1 = std::fs::File::create(&file1).unwrap();
        writeln!(f1, "hello world").unwrap();

        let file2 = dir.path().join("file2.txt");
        let mut f2 = std::fs::File::create(&file2).unwrap();
        writeln!(f2, "foo").unwrap();

        let result = count_directory_detailed(dir.path(), false);
        assert!(result.is_ok());
        let (entries, total) = result.unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(total.lines, 2);
        assert_eq!(total.words, 3); // "hello world" + "foo"
    }

    #[test]
    fn count_directory_detailed_sorted_by_path() {
        let dir = tempfile::tempdir().unwrap();

        // Create files in non-alphabetical order
        std::fs::write(dir.path().join("z_file.txt"), "z\n").unwrap();
        std::fs::write(dir.path().join("a_file.txt"), "a\n").unwrap();
        std::fs::write(dir.path().join("m_file.txt"), "m\n").unwrap();

        let result = count_directory_detailed(dir.path(), false);
        assert!(result.is_ok());
        let (entries, _) = result.unwrap();

        // Should be sorted alphabetically
        assert!(entries[0].path.to_string_lossy().contains("a_file"));
        assert!(entries[1].path.to_string_lossy().contains("m_file"));
        assert!(entries[2].path.to_string_lossy().contains("z_file"));
    }

    // Phase 5: stdin support tests
    #[test]
    fn count_from_reader_simple() {
        use std::io::Cursor;
        let reader = Cursor::new("hello world\n");
        let count = count_from_reader(reader).unwrap();
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
        assert_eq!(count.bytes, 12);
    }

    #[test]
    fn count_from_reader_empty() {
        use std::io::Cursor;
        let reader = Cursor::new("");
        let count = count_from_reader(reader).unwrap();
        assert_eq!(count.lines, 0);
        assert_eq!(count.words, 0);
        assert_eq!(count.bytes, 0);
    }

    #[test]
    fn count_from_reader_multiline() {
        use std::io::Cursor;
        // "line one\n" (9) + "line two\n" (9) + "line three\n" (11) = 29 bytes
        let reader = Cursor::new("line one\nline two\nline three\n");
        let count = count_from_reader(reader).unwrap();
        assert_eq!(count.lines, 3);
        assert_eq!(count.words, 6);
        assert_eq!(count.bytes, 29);
    }

    #[test]
    fn count_file_multiline() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "hello world").unwrap();
        writeln!(file, "foo bar baz").unwrap();
        writeln!(file, "line three").unwrap();

        let count = count_file(file.path()).unwrap();

        assert_eq!(count.lines, 3);
        // "hello world" (2) + "foo bar baz" (3) + "line three" (2) = 7 words
        assert_eq!(count.words, 7);
        // "hello world\n" (12) + "foo bar baz\n" (12) + "line three\n" (11) = 35 bytes
        assert_eq!(count.bytes, 35);
    }

    #[test]
    fn count_file_no_trailing_newline() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "hello world").unwrap(); // No newline at end

        let count = count_file(file.path()).unwrap();

        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
        assert_eq!(count.bytes, 11);
    }

    #[test]
    fn count_file_empty() {
        let file = tempfile::NamedTempFile::new().unwrap();

        let count = count_file(file.path()).unwrap();

        assert_eq!(count.lines, 0);
        assert_eq!(count.words, 0);
        assert_eq!(count.bytes, 0);
    }
}

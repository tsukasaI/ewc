use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use std::fs;
use std::io::{self, Read};
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileEntry {
    pub path: PathBuf,
    pub count: Count,
}

/// A directory-scan entry (a walked path, or a file within it) that could
/// not be counted — permission denied, vanished mid-walk, or any other I/O
/// failure — along with the error that caused it to be skipped.
pub struct SkippedEntry {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct Count {
    pub lines: u64,
    pub words: u64,
    pub bytes: u64,
    pub max_line_length: u64,
}

impl Count {
    #[cfg(test)]
    fn from_content(content: &str) -> Self {
        let mut acc = CountAccumulator::default();
        acc.write(content.as_bytes());
        acc.finish()
    }
}

// Counts lines/words/bytes over raw bytes rather than `char`s, so it works
// on arbitrary (non-UTF-8) input the same way `wc` does, and can fold a file
// in fixed-size chunks without ever buffering it whole (#5, #10). Word
// boundaries use POSIX isspace()'s ASCII whitespace set rather than
// char::is_whitespace()'s full Unicode set, since byte-level input has no
// notion of a Unicode scalar; multi-byte Unicode whitespace (e.g. U+3000,
// U+00A0) no longer splits words as a result — see README.
//
// max_line_length is the one metric measured in Unicode scalars rather than
// bytes (#15): a line of 10 Japanese characters should report 10, not the
// ~30 bytes their UTF-8 encoding takes. current_line_chars counts every byte
// that is NOT a UTF-8 continuation byte (top two bits `10`) — for valid
// UTF-8 this is exactly the scalar count, since every encoded character has
// exactly one leading (non-continuation) byte; this needs no decoding, no
// cross-chunk buffering, and degrades gracefully on malformed input (each
// byte either starts or extends a "character") rather than requiring full
// UTF-8 validation in a streaming, binary-safe counter.
#[derive(Default)]
struct CountAccumulator {
    lines: u64,
    words: u64,
    bytes: u64,
    max_line_length: u64,
    current_line_chars: u64,
    // current_line_chars alone can't tell "no bytes on this line" from "only
    // stray UTF-8 continuation bytes on this line" (both count as 0), so the
    // trailing-partial-line check in finish() needs this instead.
    line_has_content: bool,
    in_word: bool,
    // Mirrors str::lines(), which trims a lone '\r' immediately before '\n';
    // track whether the previous byte was '\r' so it can be backed out of the
    // line length once a following '\n' confirms the CRLF pair.
    prev_was_cr: bool,
}

impl CountAccumulator {
    fn write(&mut self, chunk: &[u8]) {
        self.bytes += chunk.len() as u64;

        for &b in chunk {
            if b == b'\n' {
                if self.prev_was_cr {
                    self.current_line_chars -= 1;
                }
                self.lines += 1;
                self.max_line_length = self.max_line_length.max(self.current_line_chars);
                self.current_line_chars = 0;
                self.line_has_content = false;
                self.in_word = false;
                self.prev_was_cr = false;
                continue;
            }

            self.line_has_content = true;
            if b & 0b1100_0000 != 0b1000_0000 {
                self.current_line_chars += 1;
            }
            self.prev_was_cr = b == b'\r';

            // POSIX isspace(), not u8::is_ascii_whitespace(): the latter
            // deliberately excludes vertical tab (0x0B), which would silently
            // change word-splitting behavior for content containing it.
            if matches!(b, b' ' | b'\t' | 0x0B | 0x0C | b'\r') {
                self.in_word = false;
            } else if !self.in_word {
                self.in_word = true;
                self.words += 1;
            }
        }
    }

    fn finish(mut self) -> Count {
        // A trailing partial line (content that doesn't end in '\n') still
        // counts, matching str::lines().
        if self.line_has_content {
            self.lines += 1;
            self.max_line_length = self.max_line_length.max(self.current_line_chars);
        }

        Count {
            lines: self.lines,
            words: self.words,
            bytes: self.bytes,
            max_line_length: self.max_line_length,
        }
    }
}

impl Add for Count {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            lines: self.lines.saturating_add(other.lines),
            words: self.words.saturating_add(other.words),
            bytes: self.bytes.saturating_add(other.bytes),
            max_line_length: self.max_line_length.max(other.max_line_length),
        }
    }
}

impl AddAssign for Count {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sum for Count {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, c| acc + c)
    }
}

#[derive(Debug, Clone)]
pub struct FilterConfig {
    pub include_hidden: bool,
    exclude_set: GlobSet,
    include_set: GlobSet,
}

impl Default for FilterConfig {
    fn default() -> Self {
        // Empty pattern lists can never fail to compile.
        Self::new(false, &[], &[]).expect("empty glob pattern list is always valid")
    }
}

impl FilterConfig {
    pub fn new(
        include_hidden: bool,
        exclude_patterns: &[String],
        include_patterns: &[String],
    ) -> io::Result<Self> {
        Ok(Self {
            include_hidden,
            exclude_set: Self::build_globset(exclude_patterns)?,
            include_set: Self::build_globset(include_patterns)?,
        })
    }

    fn build_globset(patterns: &[String]) -> io::Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            Self::check_pattern_bounds(pattern)?;
            let glob = Glob::new(pattern).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid glob pattern '{}': {}", pattern, e),
                )
            })?;
            builder.add(glob);
        }
        builder.build().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Failed to build glob set: {}", e),
            )
        })
    }

    /// Rejects patterns that would otherwise reach globset's `{...}`
    /// alternation compiler, which recurses once per brace-nesting level on
    /// the main thread's stack (both in building the regex and in dropping
    /// the resulting token tree) and can abort the process with a stack
    /// overflow -- a crash `Glob::new`'s `Result` can never catch or report.
    /// Measured against globset 0.4.20: regex-syntax's default nest_limit of
    /// 250 already rejects any brace nesting deeper than 124 as a graceful
    /// `Glob::new` error, so a cap of 64 loses no working pattern while
    /// staying well clear of the depth where recursion becomes dangerous.
    fn check_pattern_bounds(pattern: &str) -> io::Result<()> {
        const MAX_PATTERN_LEN: usize = 4096;
        const MAX_BRACE_DEPTH: usize = 64;

        if pattern.len() > MAX_PATTERN_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Glob pattern too long ({} bytes, max {}): '{}...'",
                    pattern.len(),
                    MAX_PATTERN_LEN,
                    &pattern[..64.min(pattern.len())]
                ),
            ));
        }

        let mut depth = 0usize;
        let mut max_depth = 0usize;
        let mut escaped = false;
        for b in pattern.bytes() {
            if escaped {
                escaped = false;
                continue;
            }
            match b {
                b'\\' => escaped = true,
                b'{' => {
                    depth += 1;
                    max_depth = max_depth.max(depth);
                }
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }

        if max_depth > MAX_BRACE_DEPTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Glob pattern brace nesting too deep ({max_depth} levels, max {MAX_BRACE_DEPTH}): '{pattern}'"
                ),
            ));
        }

        Ok(())
    }
}

// One syscall per 64KiB: large enough that wrapping in a BufReader would be
// redundant, small enough to sit comfortably on the stack under rayon's
// per-thread worker stacks during a parallel directory scan.
const READ_CHUNK_SIZE: usize = 64 * 1024;

pub fn count_file(path: &Path) -> io::Result<Count> {
    let file = fs::File::open(path)?;
    count_from_reader(file)
}

pub fn count_from_reader<R: Read>(mut reader: R) -> io::Result<Count> {
    let mut acc = CountAccumulator::default();
    let mut buf = [0u8; READ_CHUNK_SIZE];

    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        acc.write(&buf[..n]);
    }

    Ok(acc.finish())
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

fn matches_glob(glob_set: &GlobSet, relative_path: &Path) -> bool {
    let path_str = relative_path.to_string_lossy();
    glob_set.is_match(&*path_str) || glob_set.is_match(relative_path)
}

// A directory entry is pruned from the walk (not just its own file list, but
// everything beneath it) when its path matches an exclude pattern. Patterns
// like "target/*" don't match the literal string "target", so the directory
// path is also tested with a trailing separator appended, letting a
// zero-or-more "*" segment match the (otherwise absent) contents portion.
fn is_excluded_dir(relative_path: &Path, exclude_set: &GlobSet) -> bool {
    if matches_glob(exclude_set, relative_path) {
        return true;
    }
    let mut dir_str = relative_path.to_string_lossy().into_owned();
    dir_str.push('/');
    exclude_set.is_match(&dir_str)
}

// Symlinks inside a scanned directory are intentionally not followed (walkdir
// does not follow them by default): following them risks infinite loops on a
// cyclic symlink and double-counting a file reachable by more than one path.
// A broken symlink is still reported as a skipped entry, since that reflects
// a real I/O condition rather than the not-following policy.
fn walk_directory(
    path: &Path,
    config: &FilterConfig,
) -> io::Result<(Vec<PathBuf>, Vec<SkippedEntry>)> {
    let mut entries = Vec::new();
    let mut skipped = Vec::new();

    for entry in WalkDir::new(path).into_iter().filter_entry(|e| {
        if e.depth() == 0 {
            return true;
        }
        if !config.include_hidden && is_hidden(e) {
            return false;
        }
        if e.file_type().is_dir() {
            let relative_path = e.path().strip_prefix(path).unwrap_or(e.path());
            if is_excluded_dir(relative_path, &config.exclude_set) {
                return false;
            }
        }
        true
    }) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                let entry_path = e
                    .path()
                    .map_or_else(|| path.to_path_buf(), Path::to_path_buf);
                // walkdir::Error's Display already includes the path when one
                // is available ("IO error for operation on {path}: {err}"),
                // so fall back to the inner io::Error's message to avoid
                // printing the path twice in the reported warning line.
                let error = e
                    .io_error()
                    .map_or_else(|| e.to_string(), ToString::to_string);
                skipped.push(SkippedEntry {
                    path: entry_path,
                    error,
                });
                continue;
            }
        };

        let file_type = entry.file_type();
        if !file_type.is_symlink() && !file_type.is_file() {
            continue;
        }

        let file_path = entry.path();
        let relative_path = file_path.strip_prefix(path).unwrap_or(file_path);

        if matches_glob(&config.exclude_set, relative_path) {
            continue;
        }
        if !config.include_set.is_empty() && !matches_glob(&config.include_set, relative_path) {
            continue;
        }

        if file_type.is_symlink() {
            if let Err(e) = fs::metadata(file_path) {
                skipped.push(SkippedEntry {
                    path: file_path.to_path_buf(),
                    error: e.to_string(),
                });
            }
            continue;
        }

        entries.push(file_path.to_path_buf());
    }

    Ok((entries, skipped))
}

pub fn count_directory_detailed(
    path: &Path,
    config: &FilterConfig,
) -> io::Result<(Vec<FileEntry>, Count, Vec<SkippedEntry>)> {
    let (file_paths, mut skipped) = walk_directory(path, config)?;

    // Parallel file counting with rayon; failures are collected rather than
    // dropped so directory totals are never silently short (#11).
    let results: Vec<Result<FileEntry, SkippedEntry>> = file_paths
        .par_iter()
        .map(|file_path| {
            count_file(file_path)
                .map(|count| FileEntry {
                    path: file_path.clone(),
                    count,
                })
                .map_err(|e| SkippedEntry {
                    path: file_path.clone(),
                    error: e.to_string(),
                })
        })
        .collect();

    let mut entries = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(entry) => entries.push(entry),
            Err(entry) => skipped.push(entry),
        }
    }

    // Sort for deterministic output; walk order is OS-dependent, and the
    // rayon phase above preserves the (already-walked) file_paths order.
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    skipped.sort_by(|a, b| a.path.cmp(&b.path));

    let total = entries.iter().map(|e| e.count).sum();
    Ok((entries, total, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only fixture: production code only ever needs the full
    // Vec<FileEntry> from count_directory_detailed (main.rs derives
    // file_count from entries.len() itself), but most tests below only
    // care about the aggregate count/file_count/skipped, so this trims
    // count_directory_detailed's result down to that shape.
    fn count_directory(
        path: &Path,
        config: &FilterConfig,
    ) -> io::Result<(Count, usize, Vec<SkippedEntry>)> {
        let (entries, total, skipped) = count_directory_detailed(path, config)?;
        Ok((total, entries.len(), skipped))
    }

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
    fn count_file_non_utf8_succeeds() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        // 0xFF is not a valid UTF-8 lead or continuation byte anywhere.
        file.write_all(b"hello\xFF\xFEworld\n").unwrap();

        let count = count_file(file.path()).unwrap();
        assert_eq!(count.lines, 1);
        assert_eq!(count.bytes, b"hello\xFF\xFEworld\n".len() as u64);
    }

    #[test]
    fn count_from_reader_non_utf8_succeeds() {
        let data: &[u8] = b"foo\xFFbar baz\n";
        let count = count_from_reader(data).unwrap();
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
        assert_eq!(count.bytes, data.len() as u64);
    }

    #[test]
    fn count_accumulator_matches_across_chunk_boundaries() {
        // Split a CRLF pair and a single word ("bar" -> "ba" | "r", with no
        // whitespace at the split point) across separate write() calls to
        // simulate content spanning a chunked-read boundary; state carried
        // in the accumulator (current_line_chars, prev_was_cr, and crucially
        // in_word, which a per-write reset wouldn't need for the other two
        // splits) must still produce the same result as one single write()
        // of the whole content.
        let content = "hello wor\r\nld\nfoo bar";
        let whole = Count::from_content(content);
        assert_eq!(whole.words, 5); // hello, wor, ld, foo, bar

        let mut acc = CountAccumulator::default();
        for chunk in [
            b"hello wor".as_slice(),
            b"\r".as_slice(),
            b"\nld\nfoo ba".as_slice(),
            b"r".as_slice(),
        ] {
            acc.write(chunk);
        }
        let chunked = acc.finish();

        assert_eq!(chunked, whole);
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
    fn count_max_line_length_counts_multibyte_chars_not_bytes() {
        // 10 Japanese characters, 30 bytes in UTF-8 — max_line_length must
        // report 10 (chars), not 30 (bytes).
        let line = "あ".repeat(10);
        assert_eq!(line.len(), 30);
        let count = Count::from_content(&line);
        assert_eq!(count.max_line_length, 10);
    }

    #[test]
    fn count_max_line_length_mixed_ascii_and_multibyte() {
        // "café" = c,a,f,é -> 4 chars, but é is 2 bytes -> 5 bytes.
        let count = Count::from_content("café\nhi");
        assert_eq!(count.max_line_length, 4);
        assert_eq!(count.bytes, 8); // "café\nhi" = 5 + 1 + 2 bytes
    }

    #[test]
    fn count_accumulator_multibyte_char_split_across_chunk_boundary() {
        // "あ" is E3 81 82; split leading bytes from their continuation
        // bytes across four write() calls to confirm the continuation-byte
        // classification (and thus the char count) is unaffected by where
        // a chunked read happens to cut a multi-byte sequence.
        let content = "あああ"; // 3 chars, 9 bytes
        let whole = Count::from_content(content);
        assert_eq!(whole.max_line_length, 3);

        let bytes = content.as_bytes();
        let mut acc = CountAccumulator::default();
        for chunk in [&bytes[0..1], &bytes[1..4], &bytes[4..6], &bytes[6..9]] {
            acc.write(chunk);
        }
        let chunked = acc.finish();

        assert_eq!(chunked, whole);
    }

    #[test]
    fn count_crlf_line_endings() {
        // The trailing '\r' of a CRLF pair must not count toward line length,
        // matching str::lines() semantics.
        let count = Count::from_content("ab\r\nc\r\n");
        assert_eq!(count.lines, 2);
        assert_eq!(count.words, 2);
        assert_eq!(count.bytes, 7);
        assert_eq!(count.max_line_length, 2); // "ab", not "ab\r"
    }

    #[test]
    fn count_lone_carriage_return_is_whitespace_not_newline() {
        // A '\r' not followed by '\n' stays on the same line (str::lines()
        // only splits on '\n'/'\r\n'), but split_whitespace() still treats
        // it as a word separator.
        let count = Count::from_content("foo\rbar");
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
        assert_eq!(count.max_line_length, 7);
    }

    #[test]
    fn count_vertical_tab_is_word_separator() {
        // u8::is_ascii_whitespace() deliberately excludes vertical tab
        // (0x0B), unlike POSIX isspace() and char::is_whitespace(); word
        // splitting must still treat it as whitespace to match wc.
        let count = Count::from_content("a\u{B}b");
        assert_eq!(count.words, 2);
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
    fn count_add_saturates_on_overflow() {
        let count1 = Count {
            lines: u64::MAX,
            words: u64::MAX,
            bytes: u64::MAX,
            max_line_length: 0,
        };
        let count2 = Count {
            lines: 1,
            words: 1,
            bytes: 1,
            max_line_length: 0,
        };
        let total = count1 + count2;
        assert_eq!(total.lines, u64::MAX);
        assert_eq!(total.words, u64::MAX);
        assert_eq!(total.bytes, u64::MAX);

        let mut acc = count1;
        acc += count2;
        assert_eq!(acc.lines, u64::MAX);
        assert_eq!(acc.words, u64::MAX);
        assert_eq!(acc.bytes, u64::MAX);
    }

    fn default_config() -> FilterConfig {
        FilterConfig::default()
    }

    #[test]
    fn build_globset_rejects_deeply_nested_braces() {
        // 100-level-deep {a,{a,{a,...}}} alternation. Must return Err, not
        // abort the test process with a stack overflow (#34).
        let mut pattern = String::new();
        for _ in 0..100 {
            pattern.push('{');
        }
        pattern.push('a');
        for _ in 0..100 {
            pattern.push_str(",b}");
        }
        let result = FilterConfig::new(false, &[pattern], &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn build_globset_rejects_overlong_pattern() {
        let pattern = "a".repeat(5000);
        let result = FilterConfig::new(false, &[pattern], &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn build_globset_accepts_normal_brace_patterns() {
        assert!(FilterConfig::new(false, &["{a,b}".to_string()], &[]).is_ok());
        assert!(FilterConfig::new(false, &["{a,{b,c}}".to_string()], &[]).is_ok());
    }

    fn config_with_hidden() -> FilterConfig {
        FilterConfig::new(true, &[], &[]).unwrap()
    }

    #[test]
    fn count_directory_with_single_file() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "hello world").unwrap();

        let result = count_directory(dir.path(), &default_config());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.lines, 1);
        assert_eq!(count.words, 2);
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_reports_unreadable_file_as_skipped() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();

        let readable = dir.path().join("readable.txt");
        let mut f = std::fs::File::create(&readable).unwrap();
        writeln!(f, "hello world").unwrap();

        let unreadable = dir.path().join("unreadable.txt");
        std::fs::write(&unreadable, "secret").unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = count_directory(dir.path(), &default_config());
        // Not required for tempdir cleanup (unlinking only needs write access
        // to the parent directory), but restoring is good hygiene regardless.
        let restore = || {
            let _ = std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644));
        };

        let (count, file_count, skipped) = match result {
            Ok(v) => v,
            Err(e) => {
                restore();
                panic!("count_directory failed: {e}");
            }
        };
        restore();

        // Running as root (some CI/sandbox environments) bypasses permission
        // checks entirely, in which case there's nothing to assert here.
        if skipped.is_empty() && file_count == 2 {
            return;
        }

        assert_eq!(file_count, 1);
        assert_eq!(count.lines, 1);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].path, unreadable);
        assert!(!skipped[0].error.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_reports_unreadable_subdirectory_as_skipped() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();

        let readable = dir.path().join("readable.txt");
        let mut f = std::fs::File::create(&readable).unwrap();
        writeln!(f, "hello world").unwrap();

        // Without the execute bit, walkdir cannot read_dir into this
        // subdirectory at all — this exercises the walk-phase error path
        // (layer 1), distinct from a per-file read failure (layer 2), which
        // count_directory_reports_unreadable_file_as_skipped covers.
        let locked_subdir = dir.path().join("locked");
        std::fs::create_dir(&locked_subdir).unwrap();
        std::fs::write(locked_subdir.join("inside.txt"), "unreachable").unwrap();
        std::fs::set_permissions(&locked_subdir, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = count_directory(dir.path(), &default_config());
        let restore = || {
            let _ =
                std::fs::set_permissions(&locked_subdir, std::fs::Permissions::from_mode(0o755));
        };

        let (count, file_count, skipped) = match result {
            Ok(v) => v,
            Err(e) => {
                restore();
                panic!("count_directory failed: {e}");
            }
        };
        restore();

        // Running as root (some CI/sandbox environments) bypasses permission
        // checks entirely, in which case there's nothing to assert here.
        if skipped.is_empty() && file_count == 2 {
            return;
        }

        assert_eq!(file_count, 1);
        assert_eq!(count.lines, 1);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].path, locked_subdir);
        assert!(!skipped[0].error.is_empty());
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

        let result = count_directory(dir.path(), &default_config());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
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

        let result = count_directory(dir.path(), &default_config());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
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

        let result = count_directory(dir.path(), &default_config());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
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

        let result = count_directory(dir.path(), &default_config());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
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

        let result = count_directory(dir.path(), &config_with_hidden());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
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

        let result = count_directory(dir.path(), &config_with_hidden());
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
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

        let result = count_directory_detailed(dir.path(), &default_config());
        assert!(result.is_ok());
        let (entries, total, skipped) = result.unwrap();
        assert!(skipped.is_empty());

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

        let result = count_directory_detailed(dir.path(), &default_config());
        assert!(result.is_ok());
        let (entries, _, _) = result.unwrap();

        // Should be sorted alphabetically
        assert!(entries[0].path.to_string_lossy().contains("a_file"));
        assert!(entries[1].path.to_string_lossy().contains("m_file"));
        assert!(entries[2].path.to_string_lossy().contains("z_file"));
    }

    // Phase 7: exclude/include pattern tests
    #[test]
    fn count_directory_exclude_pattern() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("file.rs"), "rust code\n").unwrap();
        std::fs::write(dir.path().join("file.md"), "markdown\n").unwrap();
        std::fs::write(dir.path().join("file.txt"), "text\n").unwrap();

        let config = FilterConfig::new(false, &["*.md".to_string()], &[]).unwrap();
        let result = count_directory(dir.path(), &config);
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
        assert_eq!(file_count, 2); // .rs and .txt only
        assert_eq!(count.words, 3); // "rust code" + "text"
    }

    #[test]
    fn count_directory_include_pattern() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("file.rs"), "rust code\n").unwrap();
        std::fs::write(dir.path().join("file.md"), "markdown\n").unwrap();
        std::fs::write(dir.path().join("file.txt"), "text\n").unwrap();

        let config = FilterConfig::new(false, &[], &["*.rs".to_string()]).unwrap();
        let result = count_directory(dir.path(), &config);
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
        assert_eq!(file_count, 1); // .rs only
        assert_eq!(count.words, 2); // "rust code"
    }

    #[test]
    fn count_directory_exclude_and_include_pattern() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("main.rs"), "main\n").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "lib\n").unwrap();
        std::fs::write(dir.path().join("test_main.rs"), "test\n").unwrap();
        std::fs::write(dir.path().join("file.txt"), "text\n").unwrap();

        // Include only .rs files, but exclude test_*.rs
        let config =
            FilterConfig::new(false, &["test_*.rs".to_string()], &["*.rs".to_string()]).unwrap();
        let result = count_directory(dir.path(), &config);
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
        assert_eq!(file_count, 2); // main.rs and lib.rs only
        assert_eq!(count.words, 2); // "main" + "lib"
    }

    #[test]
    fn count_directory_exclude_directory_pattern() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();

        let subdir = dir.path().join("target");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("build.txt"), "build\n").unwrap();

        let config = FilterConfig::new(false, &["target/*".to_string()], &[]).unwrap();
        let result = count_directory(dir.path(), &config);
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
        assert_eq!(file_count, 1); // Only root.txt
        assert_eq!(count.words, 1); // "root"
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_exclude_prunes_walk_into_unreadable_subdir() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();

        // An unreadable directory under the excluded prefix would otherwise
        // surface as a skipped entry if the walk descended into it; pruning
        // the excluded directory before descending must avoid that (#70).
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let locked = target.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let config = FilterConfig::new(false, &["target/*".to_string()], &[]).unwrap();
        let result = count_directory(dir.path(), &config);
        let restore = || {
            let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));
        };

        let (count, file_count, skipped) = match result {
            Ok(v) => v,
            Err(e) => {
                restore();
                panic!("count_directory failed: {e}");
            }
        };
        restore();

        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
    }

    #[test]
    fn count_directory_exclude_prunes_bare_directory_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();

        let subdir = dir.path().join("target");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("build.txt"), "build\n").unwrap();

        // "target" (no trailing "/*") only matches the bare directory name
        // via is_excluded_dir's matches_glob branch, not the trailing-slash
        // branch exercised by count_directory_exclude_directory_pattern.
        let config = FilterConfig::new(false, &["target".to_string()], &[]).unwrap();
        let (count, file_count, skipped) = count_directory(dir.path(), &config).unwrap();

        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_include_pattern_does_not_undo_exclude_pruning() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.rs"), "root\n").unwrap();

        // An include pattern must only filter files, never un-prune a
        // directory an exclude pattern already pruned: if it did,
        // walkdir would descend into `locked` and this test would
        // surface a skipped entry instead of a clean, empty one.
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let locked = target.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let config =
            FilterConfig::new(false, &["target/*".to_string()], &["*.rs".to_string()]).unwrap();
        let result = count_directory(dir.path(), &config);
        let restore = || {
            let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));
        };

        let (count, file_count, skipped) = match result {
            Ok(v) => v,
            Err(e) => {
                restore();
                panic!("count_directory failed: {e}");
            }
        };
        restore();

        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_reports_broken_symlink_as_skipped() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();

        let broken_link = dir.path().join("dangling");
        symlink(dir.path().join("does_not_exist"), &broken_link).unwrap();

        let (count, file_count, skipped) = count_directory(dir.path(), &default_config()).unwrap();

        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].path, broken_link);
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_broken_symlink_matching_exclude_is_not_reported() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.txt"), "root\n").unwrap();

        let broken_link = dir.path().join("dangling.lock");
        symlink(dir.path().join("does_not_exist"), &broken_link).unwrap();

        let config = FilterConfig::new(false, &["*.lock".to_string()], &[]).unwrap();
        let (count, file_count, skipped) = count_directory(dir.path(), &config).unwrap();

        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_broken_symlink_failing_include_is_not_reported() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("root.rs"), "root\n").unwrap();

        let broken_link = dir.path().join("dangling.txt");
        symlink(dir.path().join("does_not_exist"), &broken_link).unwrap();

        let config = FilterConfig::new(false, &[], &["*.rs".to_string()]).unwrap();
        let (count, file_count, skipped) = count_directory(dir.path(), &config).unwrap();

        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
    }

    #[test]
    #[cfg(unix)]
    fn count_directory_silently_skips_valid_symlink() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("root.txt");
        std::fs::write(&target, "root\n").unwrap();

        let link = dir.path().join("link.txt");
        symlink(&target, &link).unwrap();

        let (count, file_count, skipped) = count_directory(dir.path(), &default_config()).unwrap();

        // The symlink is not followed and not reported: it points to a real
        // file, so this is the not-following policy, not an I/O failure.
        assert!(skipped.is_empty());
        assert_eq!(file_count, 1);
        assert_eq!(count.words, 1);
    }

    #[test]
    fn count_directory_multiple_exclude_patterns() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(dir.path().join("file.rs"), "rust\n").unwrap();
        std::fs::write(dir.path().join("file.md"), "markdown\n").unwrap();
        std::fs::write(dir.path().join("file.txt"), "text\n").unwrap();
        std::fs::write(dir.path().join("Cargo.lock"), "lock\n").unwrap();

        let config =
            FilterConfig::new(false, &["*.md".to_string(), "*.lock".to_string()], &[]).unwrap();
        let result = count_directory(dir.path(), &config);
        assert!(result.is_ok());
        let (count, file_count, skipped) = result.unwrap();
        assert!(skipped.is_empty());
        assert_eq!(file_count, 2); // .rs and .txt only
        assert_eq!(count.words, 2); // "rust" + "text"
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

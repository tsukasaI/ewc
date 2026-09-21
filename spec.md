# ewc - Enhanced Word Count

## Overview

`ewc` is an improved version of the `wc` command. It supports human-readable output format and recursive directory processing.

## Installation

```bash
cargo install ewc
```

## Basic Usage

### Single File

```bash
$ ewc file.txt
📄 file.txt
   Lines:      50
   Words:     200
   Bytes:   1,500
```

### Multiple Files

```bash
$ ewc file1.txt file2.txt
📄 file1.txt
   Lines:      50
   Words:     200
   Bytes:   1,500

📄 file2.txt
   Lines:      30
   Words:     100
   Bytes:     800

─────────────────────────
📁 Total (2 files)
   Lines:      80
   Words:     300
   Bytes:   2,300
```

### Directory (Summary)

```bash
$ ewc src/
📁 src/ (5 files)
   Lines:   1,234
   Words:   5,678
   Bytes:  45,000
```

### Directory (Verbose)

```bash
$ ewc -v src/
📄 src/main.rs        45 lines
📄 src/lib.rs        123 lines
📄 src/utils.rs       67 lines
─────────────────────────
📁 Total (3 files)   235 lines
```

## Options

| Option | Short | Description |
|--------|-------|-------------|
| `--lines` | `-l` | Show line count only |
| `--words` | `-w` | Show word count only |
| `--bytes` | `-c` | Show byte count only |
| `--max-line-length` | `-L` | Show longest line length, in characters (not bytes) |
| `--verbose` | `-v` | Show file list (directories); cannot be combined with `--compact` |
| `--all` | `-a` | Include hidden files/directories |
| `--compact` | `-C` | Single-line output; cannot be combined with `--verbose` |
| `--no-color` | - | Disable icons |
| `--json` | - | JSON output (cannot be combined with `--compact` or `--verbose`) |
| `--exclude` | - | Exclude files matching glob pattern (repeatable) |
| `--include` | - | Include only files matching glob pattern (repeatable) |

## Behavior Details

### Hidden Files

- **Default**: Files/directories starting with `.` are excluded
- **`-a` option**: Include hidden files/directories

```bash
$ ewc src/          # .gitignore, .hidden/ excluded
$ ewc -a src/       # Include all
```

### Symlinks

- Symlinks encountered while walking a directory are not followed, to
  avoid infinite loops on a cyclic symlink and double-counting a file
  reachable by more than one path
- A symlink pointing at an existing file or directory is silently skipped
  (not counted, not reported)
- A broken symlink (pointing at a path that no longer exists) is reported
  as a skipped entry, the same as any other I/O failure during the walk,
  unless it's filtered out by `--exclude`/`--include` first, in which case
  it's silently skipped like any other excluded path

### Error Handling

- Non-existent files show error message and continue
- Other files are processed normally
- Exit code 1 if any error occurs

```bash
$ ewc nofile.txt existing.txt
⚠️  nofile.txt: No such file or directory
📄 existing.txt
   Lines:      50
   Words:     200
   Bytes:   1,500
```

The warning goes to stderr and the file block to stdout; the blank line
above is only a formatting convenience in this doc, not something the
program guarantees between the two streams.

### Filenames With Control Characters

A filename containing a control character (e.g. an embedded ANSI escape
sequence, or its single-byte C1 equivalent) is displayed with those
characters replaced by U+FFFD when stdout/stderr is an actual terminal,
so it can't manipulate the terminal (move the cursor, forge extra lines,
change colors, etc.) when printed. This also applies to `clap`'s own
argument-parsing error messages, which can otherwise echo an
argument-turned-filename back verbatim. Piped or redirected output, and
`--json` (whose control-character escaping only covers C0 and is a JSON
encoding concern, not a terminal-safety one), pass the raw name through
unmodified.

### Standard Input

When no arguments are provided, or the sole argument is `-`, reads from
stdin (pipe support). A file literally named `-` can still be counted by
passing a path to it (e.g. `./-`) instead of the bare name.

```bash
$ cat file.txt | ewc
📄 <stdin>
   Lines:      50
   Words:     200
   Bytes:   1,500

$ cat file.txt | ewc -
📄 <stdin>
   Lines:      50
   Words:     200
   Bytes:   1,500
```

### JSON Mode

- `--json` cannot be combined with `--compact` or `--verbose`; neither
  affects JSON output, so combining them is rejected at argument parsing
  rather than silently ignored. `--no-color` is still meaningful with
  `--json`: JSON mode's per-failure warnings go to stderr, not stdout
  JSON, and `--no-color` suppresses the icon on those
- Exactly one input -- stdin, or a single file/directory argument -- that
  could not be opened/read at all (nonexistent path, permission denied on
  the argument itself, an unreadable stdin) emits the same single-object
  error shape, `{"file": "<name>", "error": "<message>"}`, rather than
  either a fake zeroed-count success object or the `{"files": [...],
  "total": {...}}` envelope with an empty `files` array (#46, #108). This
  also applies when an invalid `--exclude`/`--include` pattern rejects the
  run before anything is processed. Reading from stdin always produces a
  single object either way (this error shape on failure, the normal
  `{"file": "<stdin>", ...}` shape on success), since stdin is always
  exactly one input. **Not covered**: a directory argument that opens
  successfully but has some of its own entries skipped mid-walk -- that
  case still succeeds and reports via `skipped_count` below, not this
  error shape.
- A directory (or the aggregate `total` in the envelope shape) includes
  a `skipped_count` field when one or more of its *own* entries couldn't
  be counted (permission denied, vanished mid-walk, etc.), so a consumer
  parsing only stdout JSON can tell a directory's total is partial. A
  top-level file/directory argument that could not be opened at all is
  not covered by this field (it uses the error shape above when it's the
  only argument, or is simply absent from `files` in the multi-input
  envelope). The field is omitted entirely when nothing was skipped, so
  the common case's shape is unchanged

### Counting Semantics

- Counting operates on raw bytes, not decoded UTF-8 — non-UTF-8 and binary
  input is counted rather than rejected, the same way `wc` handles arbitrary
  files. Input is streamed in fixed-size chunks, so memory use doesn't scale
  with file size.
- Word boundaries use ASCII whitespace (space, tab, newline, vertical tab,
  form feed, carriage return), not Unicode whitespace — multi-byte Unicode
  whitespace characters (e.g. U+3000 ideographic space, U+00A0 no-break
  space) do not split words.
- `-L`/`--max-line-length` is the one metric measured in Unicode scalar
  values (characters) rather than bytes: a line of 10 Japanese characters
  reports 10, not the ~30 bytes their UTF-8 encoding takes. This is
  counted without decoding (a byte is counted unless it's a UTF-8
  continuation byte), so it degrades gracefully rather than erroring on
  malformed UTF-8, but it is not display-column width — wide characters
  (e.g. CJK) still count as 1, unlike GNU `wc -L`. A CRLF-terminated line
  also reports one character shorter than BSD `wc -L`: the trailing `\r`
  is excluded from the count, matching `str::lines()` semantics, whereas
  BSD `wc -L` counts it.

## Output Format

### Number Format

- Comma-separated every 3 digits
- Right-aligned (10-character width)

```
   Lines:      1,234
   Words:     12,345
   Bytes:    123,456
```

### Icons

| Icon | Meaning |
|------|---------|
| 📄 | File |
| 📁 | Directory / Total |
| ⚠️ | Error |

## Project Structure

```
ewc/
├── Cargo.toml
├── src/
│   ├── main.rs        # Entry point
│   ├── lib.rs         # Module exports
│   ├── cli.rs         # CLI options (clap)
│   ├── counter.rs     # Count logic
│   └── output.rs      # Output formatting
└── tests/
    ├── integration.rs # Integration tests
    └── benchmark.rs   # Manual performance comparison against `wc`
```

## Advanced Features

### Longest Line Length (`-L`)

The `-L` / `--max-line-length` option reports the length of the longest line,
in characters (Unicode scalar values), not bytes — see Counting Semantics.

#### Output

```bash
$ ewc -L file.txt
📄 file.txt
   Max Line:      120
```

#### With Other Metrics

```bash
$ ewc -L -l -w file.txt
📄 file.txt
   Max Line:      120
   Lines:          50
   Words:         200
```

#### Compact Mode

```bash
$ ewc -L -C file.txt
file.txt: max:120
```

#### JSON Output

```json
{
  "file": "file.txt",
  "max_line_length": 120,
  "lines": 50,
  "words": 200,
  "bytes": 1500
}
```

---

### Exclude/Include Patterns

The `--exclude` and `--include` options filter files during directory traversal using glob patterns. `--exclude` also prunes a matching directory from the walk entirely (it is not just filtered out afterward); `--include` only filters files, so it can never prune a directory `--exclude` didn't already prune.

#### Behavior

- Patterns use glob syntax (`*`, `**`, `?`, `[...]`)
- `--exclude` takes precedence over `--include`
- Multiple patterns can be specified (options are repeatable)
- Patterns match against relative paths from the walk root
- A pattern that matches a directory's path relative to the walk root
  (e.g. `--exclude target` for a top-level `target/`, or
  `--exclude "**/target"` for one at any depth) prunes that directory and
  everything under it from the walk, rather than only filtering files
  one at a time
- `*` crosses path separators the same as `**` does (no
  `literal_separator` distinction): `--exclude "*.md"` also removes
  `sub/dir/file.md`, not just top-level `.md` files
- There is no implicit basename matching: `--exclude "Cargo.lock"` does
  **not** remove `sub/Cargo.lock` (unlike `.gitignore` conventions); write
  `--exclude "**/Cargo.lock"` or `--exclude "*.lock"` to match at any depth

#### Examples

```bash
# Exclude markdown files
$ ewc --exclude "*.md" src/

# Exclude multiple patterns
$ ewc --exclude "target/*" --exclude "*.lock" .

# Include only Rust files
$ ewc --include "*.rs" src/

# Combine: only Rust files, excluding tests
$ ewc --include "*.rs" --exclude "*_test.rs" src/
```

---

### Parallel Processing

Directory scanning uses parallel file processing via `rayon` for improved performance on large directories.

#### Behavior

- Automatically parallelizes file counting in directories
- Maintains deterministic output order (files are sorted after parallel collection)
- No change for single files or stdin
- Significant speedup on directories with many files

#### Example

```bash
# Large directory benefits from parallelization
$ ewc /usr/share/
📁 /usr/share/ (10,234 files)
   Lines:    1,234,567
   Words:    5,678,901
   Bytes:  123,456,789
```

---

## License

MIT

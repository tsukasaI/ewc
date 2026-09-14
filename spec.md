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
| `--verbose` | `-v` | Show file list (directories) |
| `--all` | `-a` | Include hidden files/directories |
| `--compact` | `-C` | Single-line output |
| `--no-color` | - | Disable icons |
| `--json` | - | JSON output |
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

### Standard Input

When no arguments provided, reads from stdin (pipe support).

```bash
$ cat file.txt | ewc
📄 <stdin>
   Lines:      50
   Words:     200
   Bytes:   1,500
```

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

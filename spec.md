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
─────────────────────────────
📁 Total (3 files)   235 lines
```

## Options

| Option | Short | Description |
|--------|-------|-------------|
| `--lines` | `-l` | Show line count only |
| `--words` | `-w` | Show word count only |
| `--bytes` | `-c` | Show byte count only |
| `--verbose` | `-v` | Show file list (directories) |
| `--all` | `-a` | Include hidden files/directories |
| `--compact` | `-C` | Single-line output |
| `--no-color` | - | Disable icons |
| `--json` | - | JSON output |

## Behavior Details

### Hidden Files

- **Default**: Files/directories starting with `.` are excluded
- **`-a` option**: Include hidden files/directories

```bash
$ ewc src/          # .gitignore, .hidden/ excluded
$ ewc -a src/       # Include all
```

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

### Standard Input

When no arguments provided, reads from stdin (pipe support).

```bash
$ cat file.txt | ewc
📄 <stdin>
   Lines:      50
   Words:     200
   Bytes:   1,500
```

## Output Format

### Number Format

- Comma-separated every 3 digits
- Right-aligned (6-digit width)

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
    └── integration.rs # Integration tests
```

## License

MIT

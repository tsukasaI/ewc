# ewc - Enhanced Word Count

A modern, user-friendly alternative to the classic `wc` command, written in Rust.

## Features

- Readable output with clear labels (Lines, Words, Bytes)
- Formatted numbers with comma separators (1,234,567)
- File icons for visual clarity
- Error handling with informative messages

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Single file
$ ewc file.txt
📄 file.txt
   Lines:         42
   Words:         81
   Bytes:        956

# Lines only
$ ewc -l file.txt
📄 file.txt
   Lines:         42

# Words only
$ ewc -w file.txt
📄 file.txt
   Words:         81

# Bytes only
$ ewc -c file.txt
📄 file.txt
   Bytes:        956

# Combined options
$ ewc -lw file.txt
📄 file.txt
   Lines:         42
   Words:         81
```

## Options

```
Usage: ewc [OPTIONS] [FILE]...

Arguments:
  [FILE]...  Files to process

Options:
  -l, --lines    Show line count only
  -w, --words    Show word count only
  -c, --bytes    Show byte count only
  -h, --help     Print help
  -V, --version  Print version
```

## Development

```bash
# Run tests
cargo test

# Check for errors
cargo check

# Run the program
cargo run -- [OPTIONS] [FILE]...
```

## Project Structure

```
src/
├── main.rs      # CLI entry point
├── lib.rs       # Module exports
├── cli.rs       # Command-line argument parsing (clap)
├── counter.rs   # Word/line/byte counting logic
└── output.rs    # Output formatting
```

## License

MIT

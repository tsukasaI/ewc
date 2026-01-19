# ewc

**A modern replacement for `wc`, written in Rust.**

`ewc` is a fast, user-friendly alternative to the classic Unix `wc` command. Like [eza](https://github.com/eza-community/eza) for `ls`, [fd](https://github.com/sharkdp/fd) for `find`, and [ripgrep](https://github.com/BurntSushi/ripgrep) for `grep`, `ewc` brings a better experience to word counting.

## Why ewc?

| Feature | wc | ewc |
|---------|-----|-----|
| Human-readable output | No | Yes |
| Number formatting (1,234) | No | Yes |
| Visual file icons | No | Yes |
| Clear labels | No | Yes |
| Multiple file totals | Minimal | Clear summary |

## Example

```bash
# wc output
$ wc src/*.rs
      68     148    1798 src/main.rs
       3       9      46 src/lib.rs
     145     310    3419 src/counter.rs
     216     467    5263 total

# ewc output
$ ewc src/*.rs
📄 src/main.rs
   Lines:         68
   Words:        148
   Bytes:      1,798

📄 src/lib.rs
   Lines:          3
   Words:          9
   Bytes:         46

📄 src/counter.rs
   Lines:        145
   Words:        310
   Bytes:      3,419

─────────────────────────
📁 Total (3 files)
   Lines:        216
   Words:        467
   Bytes:      5,263
```

## Installation

```bash
# From crates.io
cargo install ewc

# From source
cargo install --path .
```

## Usage

```bash
ewc [OPTIONS] [FILE]...
```

### Options

| Option | Short | Description |
|--------|-------|-------------|
| `--lines` | `-l` | Show line count only |
| `--words` | `-w` | Show word count only |
| `--bytes` | `-c` | Show byte count only |
| `--max-line-length` | `-L` | Show longest line length |
| `--verbose` | `-v` | Show file list (directories) |
| `--all` | `-a` | Include hidden files |
| `--compact` | `-C` | Single-line output |
| `--no-color` | | Disable icons |
| `--json` | | JSON output |
| `--exclude` | | Exclude files matching glob pattern (repeatable) |
| `--include` | | Include only files matching glob pattern (repeatable) |
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |

### Examples

```bash
# Count everything
ewc file.txt

# Lines only
ewc -l file.txt

# Longest line length
ewc -L file.txt

# Multiple files (shows total)
ewc *.rs

# Directory
ewc src/

# Pipe from stdin
cat file.txt | ewc

# JSON output
ewc --json file.txt

# Filter by pattern (Rust files only)
ewc --include "*.rs" src/

# Exclude patterns
ewc --exclude "*.md" --exclude "target/*" .

# Combine include and exclude
ewc --include "*.rs" --exclude "*_test.rs" src/
```

## Contributing

### Prerequisites

- [Nix](https://nixos.org/download.html) with flakes enabled
- (Optional) [direnv](https://direnv.net/) for automatic environment loading

### Setup

```bash
# Enter dev shell (installs git hooks automatically)
nix develop

# Or with direnv
direnv allow
```

### Development

```bash
cargo test       # Run tests
cargo check      # Check for errors
cargo run -- -l file.txt  # Run locally
```

### Pre-commit Hooks

Git hooks are managed by [git-hooks.nix](https://github.com/cachix/git-hooks.nix) and run automatically:

- **rustfmt** - Code formatting
- **clippy** - Linting (warnings as errors)
- **cargo-check** - Build validation

## License

MIT

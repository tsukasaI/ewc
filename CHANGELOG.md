# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Longest line length option (`-L` / `--max-line-length`) to report the length of the longest line
- Exclude pattern option (`--exclude <PATTERN>`) to filter out files matching glob patterns during directory traversal
- Include pattern option (`--include <PATTERN>`) to only process files matching glob patterns
- Parallel file processing using `rayon` for faster directory scanning

### Dependencies

- Added `globset` for glob pattern matching
- Added `rayon` for parallel processing

## [0.4.0] - 2026-08-14

### Fixed

- Count aggregation no longer risks overflow on 32-bit targets or panics in debug builds (`u64` fields with saturating arithmetic)
- Directory scans no longer silently drop unreadable files or subdirectories from totals — skipped entries are now reported to stderr and reflected in the exit code
- `--json` output now escapes all control characters per RFC 8259 (previously only a handful were escaped, which could emit invalid JSON for filenames containing others)
- `--json` mode now reports per-file failures to stderr, and always emits a well-formed JSON document even when every input fails
- `--json` output shape is now chosen by how many arguments were given rather than how many succeeded, so e.g. two files with one failure no longer returns a different schema than two files with both failing; `--json` reading from stdin now also emits a well-formed JSON document on a read failure instead of empty stdout
- `--max-line-length`/`-L` now counts characters, not UTF-8 bytes, so non-ASCII lines are no longer over-reported

### Changed

- File counting now streams in fixed-size chunks instead of buffering whole files into memory, and no longer requires valid UTF-8 — binary and non-UTF-8 files are counted instead of erroring or being silently skipped
- Word-splitting now uses ASCII whitespace instead of full Unicode whitespace, a consequence of counting over raw bytes to support binary files
- `Count`'s public fields (`lines`, `words`, `bytes`, `max_line_length`) changed type from `usize` to `u64`

### Performance

- `Count::from_content` now does a single pass over file content instead of three

### Dependencies

- Added `serde` and `serde_json` for JSON serialization, replacing hand-rolled JSON escaping

## [0.3.2] - 2026-08-14

### Dependencies

- Updated `clap`, `globset`, `rayon`, and `tempfile` to latest compatible versions
- Removed unused `colored` dependency

## [0.3.1] - 2026-02-04

### Fixed

- Skip integration tests in Nix sandbox environment (filesystem access issues)

## [0.1.0] - 2026-01-19

### Features

- Initial release of ewc (enhanced word count)
- Human-readable output with clear labels
- Number formatting with thousands separators (1,234)
- Visual file icons
- Multiple file support with total aggregation
- Directory support with recursive file counting
- Stdin support for piped input
- Output options: `--lines`, `--words`, `--bytes`
- Display options: `--compact`, `--no-color`, `--verbose`
- Hidden files support with `--all`
- JSON output with `--json`

### Other

- Nix flake development environment
- Pre-commit hooks (rustfmt, clippy, cargo-check)
- Integration tests

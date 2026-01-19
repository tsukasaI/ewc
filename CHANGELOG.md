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

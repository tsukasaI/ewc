# CLAUDE.md — ewc

`ewc` (Enhanced Word Count) is a Rust CLI, a modern `wc` alternative with
human-readable output, JSON, glob include/exclude, and parallel directory
scanning. ~1.4k LOC (`src/`) + ~600 LOC tests. Published on crates.io,
Homebrew (`tsukasaI/ewc` tap), and a Nix flake — current version `0.3.1`
(`Cargo.toml`, `flake.nix`).

## Dev environment

Nix-flake-based. `nix develop` (or `direnv allow`) auto-installs git hooks
via `git-hooks.nix` (`flake.nix`): rustfmt, clippy (`denyWarnings = true`),
cargo-check. Plain `cargo` also works outside Nix — the hooks are the only
Nix-specific requirement.

## Commands

- `cargo test` — unit + integration tests
- `cargo check` — fast compile check
- `cargo run -- -l <file>` — run locally
- `cargo clippy -- -D warnings` — matches CI exactly
- `cargo fmt` / `cargo fmt --check`

**Nix sandbox constraint**: `nix build` / the flake's `cargoTestFlags = [
"--lib" ]` skip integration tests (`tests/integration.rs`) because the
sandbox has no filesystem access for them; unit tests (in `src/`) still run.
Commit `06c8454` ("fix(nix): skip integration tests in sandbox
environment") added this. Run `cargo test` directly (outside `nix build`)
to exercise integration tests.

## Source of truth

**README.md is user-facing truth. `spec.md` is the original design doc.**
As of this writing, `spec.md`'s options table, behavior notes, and JSON
example match the actual clap flags in `src/cli.rs` and README — no
drift found on this pass. Its "Project Structure" section is the one stale
detail: it lists only `tests/integration.rs` and omits `tests/benchmark.rs`,
which now exists. When docs disagree with code, code wins — re-diff
`spec.md` against `src/cli.rs` if either changes.

## Release flow

- Tag push (`v*`) → `.github/workflows/release.yml` cross-builds
  linux (x86_64/aarch64), macOS (x86_64/aarch64), windows (x86_64),
  generates a `SHA256SUMS` asset, and publishes release notes via
  `git-cliff` (`cliff.toml`).
- `.github/workflows/ci.yml`: separate `check`/`test`/`clippy`/`fmt` jobs.
- All GitHub Actions are pinned to commit SHAs, not tags (commit `c651d74`,
  `decision(ci)`: mitigate tag-repointing supply-chain attacks like the
  tj-actions incident). `contents: write` is scoped to the release job only.

## Design principles

- Deterministic output despite `rayon` parallelism: directory entries are
  collected via `par_iter()` then explicitly `sort_by` path
  (`src/counter.rs:167,177`) before printing.
- Dotfiles excluded by default during directory walks; `-a`/`--all`
  includes them (`is_hidden` in `src/counter.rs`).

## Re-verifying this file

- Workflow filenames/job names (`ci.yml`, `release.yml`) and the SHA pins
  in them rot on every Actions bump — re-read the files, don't trust this
  summary.
- Re-diff `spec.md` against `src/cli.rs` and README.md whenever a CLI flag
  changes; state above is current as of `dca2416`.

# CLAUDE.md — ewc

`ewc` (Enhanced Word Count) is a Rust CLI, a modern `wc` alternative with
human-readable output, JSON, glob include/exclude, and parallel directory
scanning. ~1.8k LOC (`src/`) + ~600 LOC tests (`tokei`). Published on
crates.io and a Nix flake — current version `0.4.0` (`Cargo.toml`;
`flake.nix` derives it from there via `builtins.fromTOML`). No Homebrew
tap exists; the README section that referenced one was removed, and the
stale `Formula/ewc.rb` deleted (#75), since standing up a real tap is
outside this repo's scope. `cliff.toml` was similarly stale and deleted
(#82).

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

**Nix build and integration tests**: commit `06c8454` added
`cargoTestFlags = [ "--lib" ]` on the diagnosis that the sandbox lacks
filesystem access for integration tests. That diagnosis was wrong (#62):
the surviving `--lib` unit tests already use `tempfile`/`std::fs` freely
and pass sandboxed (observed locally, not CI-checked; no workflow runs
`nix build`). The real cause was that `buildRustPackage` runs `cargo test
--profile release --target <triple>`, landing the binary at
`target/<triple>/release/ewc`, while `tests/integration.rs` looked for a
hardcoded `./target/debug/ewc` (neither that nor a bare
`./target/release/ewc` would have matched); plain `cargo test` happens to
produce the exact `./target/debug/ewc` path, masking the bug. Both
`tests/integration.rs` and `tests/benchmark.rs` were switched to
`env!("CARGO_BIN_EXE_ewc")` instead (#62, #63), and the `cargoTestFlags`
skip was removed. `tests/benchmark.rs` itself was later deleted (issue
#36, PR #135): it was never invoked by any automated path (`cargo test`
skips `#[ignore]`d tests, and no CI/Nix workflow ran it with
`--ignored`), and neither perf-motivated change since (#22, #23) touched
it or cited measurements from it.

## Source of truth

**README.md is user-facing truth. `spec.md` is the original design doc.**
As of this writing, `spec.md`'s options table, behavior notes, Project
Structure, and JSON example match the actual clap flags in `src/cli.rs`,
README, and `src/output.rs`'s formatting (width, separator length) — #83 and
#93 fixed several drifts, and #37 fixed one more (`--no-color`'s clap help
text still promised "Disable colors and icons" after colored output was
removed in #19; it now reads "Disable icons", matching README.md and
spec.md, which already described the real behavior). When docs disagree with
code, code wins — re-diff `spec.md` against `src/cli.rs` and `src/output.rs`
if either changes.

## Release flow

- Tag push (`v*`) → `.github/workflows/release.yml` cross-builds
  linux (x86_64/aarch64), macOS (x86_64/aarch64), windows (x86_64),
  generates a `SHA256SUMS` asset, attests build provenance, and publishes
  release notes via GitHub's own `generate_release_notes`. `CHANGELOG.md` is
  maintained by hand (Keep a Changelog format); `cliff.toml`, an unused
  second changelog mechanism no workflow read, has been deleted (#82).
- `.github/workflows/ci.yml`: separate `check`/`test`/`clippy`/`fmt` jobs.
- All GitHub Actions are pinned to commit SHAs, not tags (commit `c651d74`,
  `decision(ci)`: mitigate tag-repointing supply-chain attacks like the
  tj-actions incident). `contents: write` is scoped to the release job only.

## Design principles

- Deterministic output despite `rayon` parallelism: directory entries are
  collected via `par_iter()` then explicitly `sort_by` path
  (`src/counter.rs:334-335`) before printing.
- Dotfiles excluded by default during directory walks; `-a`/`--all`
  includes them (`is_hidden` in `src/counter.rs`).

## Re-verifying this file

- Workflow filenames/job names (`ci.yml`, `release.yml`) and the SHA pins
  in them rot on every Actions bump — re-read the files, don't trust this
  summary.
- Re-diff `spec.md` against `src/cli.rs` and README.md whenever a CLI flag
  changes; state above is current as of `e19ee17`. The `src/counter.rs`
  line reference above will drift once PRs touching that file merge
  (several are in flight as of this writing); re-check it rather than
  trusting the number.
- Integration tests under `nix build` have not actually been verified
  since the `--lib` skip was removed (#62): run `nix build` and confirm
  `tests/integration.rs` appears in the checkPhase log.

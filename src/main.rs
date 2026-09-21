use clap::Parser;
use std::io;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::{
    count_directory_detailed, count_file, count_from_reader, Count, FilterConfig, SkippedEntry,
};
use ewc::output::{
    format_compact_output, format_compact_total, format_json_multiple, format_json_single,
    format_output, format_separator, format_total_output, format_verbose_output, icon,
    sanitize_for_display, JsonFileResult, OutputKind,
};

const WARNING_ICON: &str = "\u{26A0}\u{FE0F}  ";

struct ProcessResult {
    count: Count,
    file_count: usize,
    skipped: Vec<SkippedEntry>,
    is_directory: bool,
}

fn process_path(path: &Path, config: &FilterConfig) -> io::Result<ProcessResult> {
    if path.is_dir() {
        let (entries, count, skipped) = count_directory_detailed(path, config)?;
        Ok(ProcessResult {
            count,
            file_count: entries.len(),
            skipped,
            is_directory: true,
        })
    } else {
        let count = count_file(path)?;
        Ok(ProcessResult {
            count,
            file_count: 1,
            skipped: Vec::new(),
            is_directory: false,
        })
    }
}

/// Writes a top-level "file/directory could not be processed" warning.
fn warn_file_error(
    err: &mut impl Write,
    file: &Path,
    e: &io::Error,
    no_color: bool,
) -> io::Result<()> {
    let is_tty = io::stderr().is_terminal();
    writeln!(
        err,
        "{}{}: {e}",
        icon(no_color, WARNING_ICON),
        sanitize_for_display(&file.to_string_lossy(), is_tty)
    )
}

/// Writes one warning line per skipped entry, matching warn_file_error's
/// style. Returns whether anything was skipped, so callers can fold it
/// into the process's exit code.
fn report_skipped(
    err: &mut impl Write,
    skipped: &[SkippedEntry],
    no_color: bool,
) -> io::Result<bool> {
    let is_tty = io::stderr().is_terminal();
    let warning = icon(no_color, WARNING_ICON);
    for entry in skipped {
        let path_str = entry.path.display().to_string();
        writeln!(
            err,
            "{warning}{}: {}",
            sanitize_for_display(&path_str, is_tty),
            entry.error
        )?;
    }
    Ok(!skipped.is_empty())
}

fn create_filter_config(args: &Args) -> io::Result<FilterConfig> {
    FilterConfig::new(args.all, &args.exclude, &args.include)
}

/// Formats an invalid --exclude/--include glob-pattern error for stderr.
/// The error's Display embeds the offending pattern (both this wrapper's
/// own message and globset's inner echo of it), so it needs the same
/// terminal sanitization as any other argument-derived output.
fn filter_config_error_line(e: &io::Error, no_color: bool, is_tty: bool) -> String {
    format!(
        "{}{}",
        icon(no_color, WARNING_ICON),
        sanitize_for_display(&e.to_string(), is_tty)
    )
}

/// Sanitizes every element of `argv` and re-parses it.
///
/// Sanitizing argv and re-parsing, rather than sanitizing clap's
/// already-rendered error text, matters because a raw newline in an
/// argument would otherwise reach `.lines()` and split into a second,
/// forged-looking line before any per-line sanitizer ever saw it.
fn reparse_sanitized_argv(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Args, clap::Error> {
    let clean_args: Vec<String> = argv
        .into_iter()
        .map(|a| sanitize_for_display(&a.to_string_lossy(), true).into_owned())
        .collect();
    Args::try_parse_from(clean_args)
}

/// Exits with a sanitized rendering of a clap parse error.
fn exit_with_sanitized_parse_error(
    original: clap::Error,
    argv: impl IntoIterator<Item = std::ffi::OsString>,
) -> ! {
    match reparse_sanitized_argv(argv) {
        Err(clean_err) if clean_err.use_stderr() => clean_err.exit(),
        _ => {
            // Sanitized argv unexpectedly parsed cleanly (or hit
            // help/version); fall back to the original error as a
            // fail-closed path. Sanitized as a single string rather than
            // split into lines first: the rendered text can't distinguish
            // clap's own newlines from argument-derived ones, so splitting
            // on them first would reopen the same forgery this function
            // exists to close.
            eprintln!(
                "{}",
                sanitize_for_display(&original.render().to_string(), true)
            );
            process::exit(original.exit_code());
        }
    }
}

fn main() {
    let args = match Args::try_parse() {
        Ok(args) => args,
        // A parse error can echo back an argument verbatim (e.g. an
        // unrecognized flag that's actually a filename from shell glob
        // expansion), so it needs the same terminal sanitization as any
        // other filename-derived output; --help/--version (use_stderr() is
        // false there) go through clap's own exit() unchanged.
        Err(e) if e.use_stderr() => {
            if io::stderr().is_terminal() {
                exit_with_sanitized_parse_error(e, std::env::args_os());
            } else {
                e.exit();
            }
        }
        Err(e) => e.exit(),
    };

    // Built once, before mode dispatch, so an invalid --exclude/--include
    // pattern is caught even in stdin mode instead of being silently ignored.
    let config = match create_filter_config(&args) {
        Ok(config) => config,
        Err(e) => {
            let is_tty = io::stderr().is_terminal();
            // Best-effort: the process is exiting on this path regardless,
            // so a write failure here (including a broken pipe) has no
            // further action to take.
            let _ = writeln!(
                io::stderr(),
                "{}",
                filter_config_error_line(&e, args.no_color, is_tty)
            );
            if args.json {
                // Keep stdout valid JSON even on failure, matching
                // run_json_mode's all-inputs-failed behavior.
                let _ = writeln!(
                    io::stdout(),
                    "{}",
                    format_json_multiple(&[], &Count::default())
                );
            }
            process::exit(1);
        }
    };

    // A single bare "-" is a request to read stdin, matching wc.
    let reads_stdin =
        args.files.is_empty() || (args.files.len() == 1 && args.files[0].as_os_str() == "-");

    let result = if reads_stdin {
        run_stdin_mode(&args)
    } else if args.json {
        run_json_mode(&args, &config)
    } else {
        run_normal_mode(&args, &config)
    };

    match result {
        Ok(has_error) => {
            if has_error {
                process::exit(1);
            }
        }
        // A downstream reader (e.g. `| head`) closing early is not a
        // program failure; exit the way a SIGPIPE-killed process would
        // (matching GNU/BSD wc under `set -o pipefail`), with no further
        // output to the dead pipe.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => process::exit(141),
        Err(e) => {
            // Best-effort, matching the other top-level error paths: if
            // stderr is itself unwritable here there is no further action
            // to take, and panicking would defeat the point of this match.
            let _ = writeln!(io::stderr(), "ewc: {e}");
            process::exit(1);
        }
    }
}

fn run_stdin_mode(args: &Args) -> io::Result<bool> {
    let is_terminal = io::stdout().is_terminal();
    let mut out = io::stdout().lock();
    let mut err = io::stderr().lock();

    let count = match count_from_reader(io::stdin().lock()) {
        Ok(c) => c,
        Err(e) => {
            writeln!(err, "{}<stdin>: {e}", icon(args.no_color, WARNING_ICON))?;
            if args.json {
                // Same bare-object shape as the success path below (a zeroed
                // Count on failure), not the {files, total} envelope: stdin
                // is always exactly one input, so its JSON shape shouldn't
                // depend on whether reading it happened to succeed (#85).
                let result = JsonFileResult {
                    name: "<stdin>".to_string(),
                    count: Count::default(),
                    kind: OutputKind::File,
                    skipped_count: 0,
                };
                writeln!(out, "{}", format_json_single(&result))?;
            }
            return Ok(true);
        }
    };

    if args.json {
        let result = JsonFileResult {
            name: "<stdin>".to_string(),
            count,
            kind: OutputKind::File,
            skipped_count: 0,
        };
        writeln!(out, "{}", format_json_single(&result))?;
    } else if args.compact {
        writeln!(
            out,
            "{}",
            format_compact_output("<stdin>", &count, OutputKind::File, args, is_terminal)
        )?;
    } else {
        writeln!(
            out,
            "{}",
            format_output("<stdin>", &count, OutputKind::File, args, is_terminal)
        )?;
    }
    Ok(false)
}

fn run_json_mode(args: &Args, config: &FilterConfig) -> io::Result<bool> {
    let mut out = io::stdout().lock();
    let mut err = io::stderr().lock();
    let mut results: Vec<JsonFileResult> = Vec::new();
    let mut total_count = Count::default();
    let mut has_error = false;

    for file in &args.files {
        let path = file.as_path();
        let result = match process_path(path, config) {
            Ok(result) => result,
            Err(e) => {
                warn_file_error(&mut err, file, &e, args.no_color)?;
                has_error = true;
                continue;
            }
        };

        if report_skipped(&mut err, &result.skipped, args.no_color)? {
            has_error = true;
        }

        let kind = if result.is_directory {
            OutputKind::Directory(result.file_count)
        } else {
            OutputKind::File
        };
        results.push(JsonFileResult {
            name: file.to_string_lossy().into_owned(),
            count: result.count,
            kind,
            skipped_count: result.skipped.len(),
        });
        total_count += result.count;
    }

    // Shape is chosen by how many arguments were given, not how many
    // succeeded: otherwise `ewc --json good.txt bad.txt` (1 success of 2
    // args) would return the bare single-object shape while
    // `ewc --json bad1.txt bad2.txt` (0 successes) returns the {files,
    // total} envelope — an unpredictable schema for a pipe consumer that
    // can't know success counts ahead of time (#27).
    match (args.files.len(), results.as_slice()) {
        (1, [single]) => writeln!(out, "{}", format_json_single(single))?,
        _ => writeln!(out, "{}", format_json_multiple(&results, &total_count))?,
    }

    Ok(has_error)
}

fn run_normal_mode(args: &Args, config: &FilterConfig) -> io::Result<bool> {
    let mut out = io::stdout().lock();
    let mut err = io::stderr().lock();
    let mut has_error = false;
    let mut total_count = Count::default();
    let mut total_file_count = 0;
    let mut successful_args = 0;
    let is_terminal = io::stdout().is_terminal();
    // Blank separator owed before the next block. A failing argument
    // prints nothing, so this can't be derived from loop position (#49).
    let mut needs_leading_blank = false;

    for file in &args.files {
        let path = file.as_path();

        if path.is_dir() && args.verbose {
            match count_directory_detailed(path, config) {
                Ok((entries, dir_total, skipped)) => {
                    if needs_leading_blank {
                        writeln!(out)?;
                    }
                    writeln!(
                        out,
                        "{}",
                        format_verbose_output(&entries, &dir_total, args, is_terminal)
                    )?;

                    if report_skipped(&mut err, &skipped, args.no_color)? {
                        has_error = true;
                    }

                    total_count += dir_total;
                    total_file_count += entries.len();
                    successful_args += 1;
                    needs_leading_blank = !args.compact;
                }
                Err(e) => {
                    warn_file_error(&mut err, file, &e, args.no_color)?;
                    has_error = true;
                }
            }
        } else {
            match process_path(path, config) {
                Ok(result) => {
                    let kind = if result.is_directory {
                        OutputKind::Directory(result.file_count)
                    } else {
                        OutputKind::File
                    };
                    let name = file.to_string_lossy();
                    let output = if args.compact {
                        format_compact_output(&name, &result.count, kind, args, is_terminal)
                    } else {
                        format_output(&name, &result.count, kind, args, is_terminal)
                    };
                    if needs_leading_blank {
                        writeln!(out)?;
                    }
                    writeln!(out, "{output}")?;

                    if report_skipped(&mut err, &result.skipped, args.no_color)? {
                        has_error = true;
                    }

                    total_count += result.count;
                    total_file_count += result.file_count;
                    successful_args += 1;
                    needs_leading_blank = !args.compact;
                }
                Err(e) => {
                    warn_file_error(&mut err, file, &e, args.no_color)?;
                    has_error = true;
                }
            }
        }
    }

    if successful_args > 1 {
        if !args.compact {
            if needs_leading_blank {
                writeln!(out)?;
            }
            writeln!(out, "{}", format_separator())?;
        }
        let total = if args.compact {
            format_compact_total(total_file_count, &total_count, args)
        } else {
            format_total_output(total_file_count, &total_count, args)
        };
        writeln!(out, "{total}")?;
    }

    Ok(has_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn invalid_glob_pattern_error_is_sanitized() {
        let e = FilterConfig::new(false, &["[\nerror: FORGED GLOB".to_string()], &[]).unwrap_err();
        let line = filter_config_error_line(&e, false, true);
        assert!(!line.contains('\n'));
        assert!(line.contains('\u{FFFD}'));
    }

    #[test]
    fn reparse_sanitized_argv_removes_embedded_newlines() {
        // A raw newline in an argument, once echoed into clap's rendered
        // error text, would look like a second, forged line of output.
        // reparse_sanitized_argv sanitizes argv *before* clap ever sees it,
        // so the newline can't reach the parser (and therefore can't reach
        // the rendered error) in the first place.
        let argv = [
            OsString::from("ewc"),
            OsString::from("--bogus\nerror: FORGED LINE"),
        ];
        let err = reparse_sanitized_argv(argv).unwrap_err();
        let rendered = err.render().to_string();
        // The forged text can still appear (it's just an odd flag value),
        // but never as a line of its own: the newline that would have
        // split it out was replaced before clap ever parsed the argument.
        assert!(!rendered.contains("\nerror: FORGED LINE"));
    }
}

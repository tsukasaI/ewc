use clap::Parser;
use std::io;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::{
    count_directory_detailed, count_file, count_from_reader, Count, FileEntry, FilterConfig,
    SkippedEntry,
};
use ewc::output::{
    format_compact_output, format_compact_total, format_json_error, format_json_multiple,
    format_json_single, format_output, format_separator, format_total_output,
    format_verbose_output, icon, sanitize_for_display, JsonFileResult, OutputKind,
};

const WARNING_ICON: &str = "\u{26A0}\u{FE0F}  ";

struct ProcessResult {
    count: Count,
    file_count: usize,
    skipped: Vec<SkippedEntry>,
    is_directory: bool,
    /// `Some` only for a directory processed with `verbose: true`; carries
    /// the per-file breakdown format_verbose_output needs. `None` for a
    /// file, or for a directory processed without --verbose, where only
    /// the aggregate count/file_count are needed.
    entries: Option<Vec<FileEntry>>,
}

fn process_path(path: &Path, config: &FilterConfig, verbose: bool) -> io::Result<ProcessResult> {
    if !path.is_dir() {
        return Ok(ProcessResult {
            count: count_file(path)?,
            file_count: 1,
            skipped: Vec::new(),
            is_directory: false,
            entries: None,
        });
    }

    let (entries, count, skipped) = count_directory_detailed(path, config)?;
    Ok(ProcessResult {
        count,
        file_count: entries.len(),
        skipped,
        is_directory: true,
        entries: verbose.then_some(entries),
    })
}

/// Formats a processed argument's output line(s): the per-file verbose
/// breakdown when `result.entries` is set, otherwise the normal or compact
/// single-block form.
fn format_process_result(
    name: &str,
    result: &ProcessResult,
    args: &Args,
    is_terminal: bool,
) -> String {
    if let Some(entries) = &result.entries {
        return format_verbose_output(entries, &result.count, args, is_terminal);
    }
    let kind = if result.is_directory {
        OutputKind::Directory(result.file_count)
    } else {
        OutputKind::File
    };
    if args.compact {
        format_compact_output(name, &result.count, kind, args, is_terminal)
    } else {
        format_output(name, &result.count, kind, args, is_terminal)
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
                // Keep stdout valid JSON even on failure. A single input
                // (stdin, or one file/directory argument) uses the same
                // error shape a single failing input uses everywhere else
                // (#46, #108); more than one argument uses the multi-input
                // envelope, matching run_json_mode's all-inputs-failed
                // behavior.
                let single_input_name = match args.files.as_slice() {
                    [] => Some("<stdin>".to_string()),
                    [only] if only.as_os_str() == "-" => Some("<stdin>".to_string()),
                    [only] => Some(only.to_string_lossy().into_owned()),
                    _ => None,
                };
                let json = match single_input_name {
                    Some(name) => format_json_error(&name, &e.to_string()),
                    None => format_json_multiple(&[], &Count::default()),
                };
                let _ = writeln!(io::stdout(), "{json}");
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
                // A real error object, not the {files, total} envelope and
                // not a fake zeroed-count success object: stdin is always
                // exactly one input, so its JSON failure shape is the same
                // one a single failing file/directory argument uses (#46,
                // #108).
                writeln!(out, "{}", format_json_error("<stdin>", &e.to_string()))?;
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
    // Set only when the single argument (if there is exactly one) fails,
    // so a single-argument failure can emit the same error shape stdin
    // uses instead of falling back to the multi-input envelope (#46, #108).
    let mut single_arg_error: Option<String> = None;

    for file in &args.files {
        let path = file.as_path();
        let result = match process_path(path, config, false) {
            Ok(result) => result,
            Err(e) => {
                if args.files.len() == 1 {
                    single_arg_error = Some(e.to_string());
                }
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
    // can't know success counts ahead of time (#27). A single argument that
    // failed entirely gets the same error-object shape stdin's read
    // failure uses, rather than the multi-input envelope with an empty
    // `files` array (#46, #108).
    if let Some(error) = single_arg_error {
        let name = args.files[0].to_string_lossy();
        writeln!(out, "{}", format_json_error(&name, &error))?;
    } else {
        match (args.files.len(), results.as_slice()) {
            (1, [single]) => writeln!(out, "{}", format_json_single(single))?,
            _ => writeln!(out, "{}", format_json_multiple(&results, &total_count))?,
        }
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
        let result = match process_path(path, config, args.verbose) {
            Ok(result) => result,
            Err(e) => {
                warn_file_error(&mut err, file, &e, args.no_color)?;
                has_error = true;
                continue;
            }
        };

        let name = file.to_string_lossy();
        if needs_leading_blank {
            writeln!(out)?;
        }
        writeln!(
            out,
            "{}",
            format_process_result(&name, &result, args, is_terminal)
        )?;

        if report_skipped(&mut err, &result.skipped, args.no_color)? {
            has_error = true;
        }

        total_count += result.count;
        total_file_count += result.file_count;
        successful_args += 1;
        needs_leading_blank = !args.compact;
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

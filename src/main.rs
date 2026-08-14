use clap::Parser;
use std::io;
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::{
    count_directory, count_directory_detailed, count_file, count_from_reader, Count, FilterConfig,
    SkippedEntry,
};
use ewc::output::{
    format_compact_output, format_compact_total, format_json_multiple, format_json_single,
    format_output, format_separator, format_total_output, format_verbose_output, JsonFileResult,
    OutputKind,
};

const WARNING_ICON: &str = "\u{26A0}\u{FE0F}";

struct ProcessResult {
    count: Count,
    file_count: usize,
    skipped: Vec<SkippedEntry>,
}

fn process_path(path: &Path, config: &FilterConfig) -> io::Result<ProcessResult> {
    if path.is_dir() {
        let (count, file_count, skipped) = count_directory(path, config)?;
        Ok(ProcessResult {
            count,
            file_count,
            skipped,
        })
    } else {
        let count = count_file(path)?;
        Ok(ProcessResult {
            count,
            file_count: 1,
            skipped: Vec::new(),
        })
    }
}

/// Prints one warning line per skipped entry, matching the existing
/// top-level-failure style. Returns whether anything was skipped, so callers
/// can fold it into the process's exit code.
fn report_skipped(skipped: &[SkippedEntry]) -> bool {
    for entry in skipped {
        eprintln!("{WARNING_ICON}  {}: {}", entry.path.display(), entry.error);
    }
    !skipped.is_empty()
}

fn create_filter_config(args: &Args) -> FilterConfig {
    FilterConfig::new(args.all, args.exclude.clone(), args.include.clone())
}

fn main() {
    let args = Args::parse();

    if args.files.is_empty() {
        run_stdin_mode(&args);
    } else if args.json {
        run_json_mode(&args);
    } else {
        run_normal_mode(&args);
    }
}

fn run_stdin_mode(args: &Args) {
    let count = match count_from_reader(io::stdin().lock()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{WARNING_ICON}  <stdin>: {e}");
            process::exit(1);
        }
    };

    if args.json {
        let result = JsonFileResult {
            name: "<stdin>".to_string(),
            count,
            is_directory: false,
            file_count: None,
        };
        println!("{}", format_json_single(&result));
    } else if args.compact {
        println!(
            "{}",
            format_compact_output("<stdin>", &count, OutputKind::File, args)
        );
    } else {
        println!(
            "{}",
            format_output("<stdin>", &count, OutputKind::File, args)
        );
    }
}

fn run_json_mode(args: &Args) {
    let mut results: Vec<JsonFileResult> = Vec::new();
    let mut total_count = Count::default();
    let mut has_error = false;
    let config = create_filter_config(args);

    for file in &args.files {
        let path = Path::new(file);
        let result = match process_path(path, &config) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{WARNING_ICON}  {file}: {e}");
                has_error = true;
                continue;
            }
        };

        if report_skipped(&result.skipped) {
            has_error = true;
        }

        let is_directory = path.is_dir();
        results.push(JsonFileResult {
            name: file.clone(),
            count: result.count,
            is_directory,
            file_count: is_directory.then_some(result.file_count),
        });
        total_count += result.count;
    }

    match results.as_slice() {
        [single] => println!("{}", format_json_single(single)),
        _ => println!("{}", format_json_multiple(&results, &total_count)),
    }

    if has_error {
        process::exit(1);
    }
}

fn run_normal_mode(args: &Args) {
    let mut has_error = false;
    let mut total_count = Count::default();
    let mut total_file_count = 0;
    let mut successful_args = 0;
    let file_count = args.files.len();
    let config = create_filter_config(args);

    for (index, file) in args.files.iter().enumerate() {
        let path = Path::new(file);
        let is_last = index == file_count - 1;

        if path.is_dir() && args.verbose {
            match count_directory_detailed(path, &config) {
                Ok((entries, dir_total, skipped)) => {
                    println!("{}", format_verbose_output(&entries, &dir_total, args));

                    if report_skipped(&skipped) {
                        has_error = true;
                    }

                    total_count += dir_total;
                    total_file_count += entries.len();
                    successful_args += 1;

                    if !is_last {
                        println!();
                    }
                }
                Err(e) => {
                    eprintln!("{WARNING_ICON}  {file}: {e}");
                    has_error = true;
                }
            }
        } else {
            match process_path(path, &config) {
                Ok(result) => {
                    let kind = if path.is_dir() {
                        OutputKind::Directory(result.file_count)
                    } else {
                        OutputKind::File
                    };
                    let output = if args.compact {
                        format_compact_output(file, &result.count, kind, args)
                    } else {
                        format_output(file, &result.count, kind, args)
                    };
                    println!("{output}");

                    if report_skipped(&result.skipped) {
                        has_error = true;
                    }

                    total_count += result.count;
                    total_file_count += result.file_count;
                    successful_args += 1;

                    if !args.compact && !is_last {
                        println!();
                    }
                }
                Err(e) => {
                    eprintln!("{WARNING_ICON}  {file}: {e}");
                    has_error = true;
                }
            }
        }
    }

    if successful_args > 1 {
        if !args.compact {
            println!();
            println!("{}", format_separator());
        }
        let total = if args.compact {
            format_compact_total(total_file_count, &total_count, args)
        } else {
            format_total_output(total_file_count, &total_count, args)
        };
        println!("{total}");
    }

    if has_error {
        process::exit(1);
    }
}

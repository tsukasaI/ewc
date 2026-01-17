use clap::Parser;
use std::io;
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::{count_directory, count_directory_detailed, count_file, Count};
use ewc::output::{
    format_compact_output, format_compact_total, format_json_multiple, format_json_single,
    format_output, format_separator, format_total_output, format_verbose_output, JsonFileResult,
    OutputKind,
};

const WARNING_ICON: &str = "\u{26A0}\u{FE0F}";

struct ProcessResult {
    count: Count,
    file_count: usize,
}

fn process_path(path: &Path, include_hidden: bool) -> io::Result<ProcessResult> {
    if path.is_dir() {
        let (count, file_count) = count_directory(path, include_hidden)?;
        Ok(ProcessResult { count, file_count })
    } else {
        let count = count_file(path)?;
        Ok(ProcessResult {
            count,
            file_count: 1,
        })
    }
}

fn main() {
    let args = Args::parse();

    if args.files.is_empty() {
        eprintln!("ewc: No files specified");
        process::exit(1);
    }

    // JSON mode requires buffering results
    if args.json {
        run_json_mode(&args);
    } else {
        run_normal_mode(&args);
    }
}

fn run_json_mode(args: &Args) {
    let mut results: Vec<JsonFileResult> = Vec::new();
    let mut total_count = Count::default();
    let mut has_error = false;

    for file in &args.files {
        let path = Path::new(file);
        let Ok(result) = process_path(path, args.all) else {
            has_error = true;
            continue;
        };

        let is_directory = path.is_dir();
        results.push(JsonFileResult {
            name: file.clone(),
            count: result.count.clone(),
            is_directory,
            file_count: is_directory.then_some(result.file_count),
        });
        total_count += result.count;
    }

    match results.as_slice() {
        [] => {}
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

    for (index, file) in args.files.iter().enumerate() {
        let path = Path::new(file);
        let is_last = index == file_count - 1;

        if path.is_dir() && args.verbose {
            match count_directory_detailed(path, args.all) {
                Ok((entries, dir_total)) => {
                    println!("{}", format_verbose_output(&entries, &dir_total, args));

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
            match process_path(path, args.all) {
                Ok(result) => {
                    let kind = match path.is_dir() {
                        true => OutputKind::Directory(result.file_count),
                        false => OutputKind::File,
                    };
                    let format_fn = if args.compact {
                        format_compact_output
                    } else {
                        format_output
                    };
                    println!("{}", format_fn(file, &result.count, kind, args));

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

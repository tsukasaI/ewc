use clap::Parser;
use std::io;
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::{count_directory, count_file, Count};
use ewc::output::{format_output, format_separator, format_total_output, OutputKind};

struct ProcessResult {
    count: Count,
    file_count: usize,
}

fn process_path(path: &Path) -> io::Result<ProcessResult> {
    if path.is_dir() {
        let (count, file_count) = count_directory(path)?;
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

    let mut has_error = false;
    let mut total_count = Count::default();
    let mut total_file_count = 0;
    let mut successful_args = 0;
    let is_last = |index: usize| index == args.files.len() - 1;

    for (index, file) in args.files.iter().enumerate() {
        let path = Path::new(file);

        match process_path(path) {
            Ok(result) => {
                let kind = if path.is_dir() {
                    OutputKind::Directory(result.file_count)
                } else {
                    OutputKind::File
                };
                let output = format_output(file, &result.count, kind, &args);
                println!("{output}");

                total_count += result.count;
                total_file_count += result.file_count;
                successful_args += 1;

                if !is_last(index) {
                    println!();
                }
            }
            Err(e) => {
                eprintln!("\u{26A0}\u{FE0F}  {file}: {e}");
                has_error = true;
            }
        }
    }

    if successful_args > 1 {
        println!();
        println!("{}", format_separator());
        println!(
            "{}",
            format_total_output(total_file_count, &total_count, &args)
        );
    }

    if has_error {
        process::exit(1);
    }
}

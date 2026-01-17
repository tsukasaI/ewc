use clap::Parser;
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::{count_file, Count};
use ewc::output::{format_file_output, format_separator, format_total_output};

fn main() {
    let args = Args::parse();

    if args.files.is_empty() {
        eprintln!("ewc: No files specified");
        process::exit(1);
    }

    let mut has_error = false;
    let mut total_count = Count::default();
    let mut successful_file_count = 0;

    for (index, file) in args.files.iter().enumerate() {
        let path = Path::new(file);
        match count_file(path) {
            Ok(count) => {
                let output = format_file_output(
                    file,
                    &count,
                    args.show_lines(),
                    args.show_words(),
                    args.show_bytes(),
                );
                println!("{output}");

                total_count += count;
                successful_file_count += 1;

                // Blank line between files (not after last file)
                if index < args.files.len() - 1 {
                    println!();
                }
            }
            Err(e) => {
                eprintln!("\u{26A0}\u{FE0F}  {file}: {e}");
                has_error = true;
            }
        }
    }

    // Show total only if 2+ files succeeded
    if successful_file_count > 1 {
        println!();
        println!("{}", format_separator());
        println!(
            "{}",
            format_total_output(
                successful_file_count,
                &total_count,
                args.show_lines(),
                args.show_words(),
                args.show_bytes(),
            )
        );
    }

    if has_error {
        process::exit(1);
    }
}

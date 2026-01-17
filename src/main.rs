use clap::Parser;
use std::path::Path;
use std::process;

use ewc::cli::Args;
use ewc::counter::count_file;
use ewc::output::format_file_output;

fn main() {
    let args = Args::parse();

    if args.files.is_empty() {
        eprintln!("ewc: No files specified");
        process::exit(1);
    }

    let mut has_error = false;

    for file in &args.files {
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
                println!("{}", output);
            }
            Err(e) => {
                eprintln!("\u{26A0}\u{FE0F}  {}: {}", file, e);
                has_error = true;
            }
        }
    }

    if has_error {
        process::exit(1);
    }
}

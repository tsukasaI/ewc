use std::process::Command;
use std::time::Instant;

fn create_test_file(lines: usize) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    for _ in 0..lines {
        writeln!(file, "hello world test line for benchmark").unwrap();
    }
    file
}

fn bench_wc(path: &str, runs: u32) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..runs {
        Command::new("wc")
            .arg(path)
            .output()
            .expect("failed to run wc");
    }
    start.elapsed()
}

fn bench_ewc(path: &str, runs: u32) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..runs {
        Command::new("./target/release/ewc")
            .arg(path)
            .output()
            .expect("failed to run ewc");
    }
    start.elapsed()
}

#[test]
#[ignore] // Run with: cargo test --release benchmark -- --ignored --nocapture
fn benchmark_comparison() {
    println!("\n=== ewc vs wc Benchmark ===\n");

    // Small file (1K lines)
    let small = create_test_file(1_000);
    let runs = 20;

    println!("Small file (1K lines) - {} runs:", runs);
    let wc_time = bench_wc(small.path().to_str().unwrap(), runs);
    let ewc_time = bench_ewc(small.path().to_str().unwrap(), runs);
    println!("  wc:  {:?} ({:.2?} per run)", wc_time, wc_time / runs);
    println!("  ewc: {:?} ({:.2?} per run)", ewc_time, ewc_time / runs);
    println!("  ratio: {:.2}x", ewc_time.as_secs_f64() / wc_time.as_secs_f64());

    // Medium file (100K lines)
    let medium = create_test_file(100_000);
    let runs = 10;

    println!("\nMedium file (100K lines) - {} runs:", runs);
    let wc_time = bench_wc(medium.path().to_str().unwrap(), runs);
    let ewc_time = bench_ewc(medium.path().to_str().unwrap(), runs);
    println!("  wc:  {:?} ({:.2?} per run)", wc_time, wc_time / runs);
    println!("  ewc: {:?} ({:.2?} per run)", ewc_time, ewc_time / runs);
    println!("  ratio: {:.2}x", ewc_time.as_secs_f64() / wc_time.as_secs_f64());

    // Large file (500K lines)
    let large = create_test_file(500_000);
    let runs = 5;

    println!("\nLarge file (500K lines) - {} runs:", runs);
    let wc_time = bench_wc(large.path().to_str().unwrap(), runs);
    let ewc_time = bench_ewc(large.path().to_str().unwrap(), runs);
    println!("  wc:  {:?} ({:.2?} per run)", wc_time, wc_time / runs);
    println!("  ewc: {:?} ({:.2?} per run)", ewc_time, ewc_time / runs);
    println!("  ratio: {:.2}x", ewc_time.as_secs_f64() / wc_time.as_secs_f64());

    println!("\n(ratio < 1.0 means ewc is faster)");
}

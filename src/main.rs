use std::{
    process::{exit, Command},
    sync::mpsc::Receiver,
    time::Duration,
};

use clap::Parser;

/// Continuously collects memory samples from a running process and returns the peak memory usage
/// in kilobytes.
///
/// # Arguments
///
/// * `binary_name` - The name of the binary being executed
/// * `pid` - The binary's pid
/// * `sample_rate_ms` - The sample rate in milliseconds, which is the time the sampling thread
/// waits until collecting another sample.
/// * `timeout_ms` - The time to wait before starting to collect samples in milliseconds.
/// * `receiver` - A receiver which receives a message from the main thread when the child process
/// returns.
fn sample_loop(
    pid: u32,
    sample_rate_ms: usize,
    timeout_ms: usize,
    receiver: Receiver<()>,
) -> usize {
    std::thread::sleep(Duration::from_millis(timeout_ms as u64));
    let mut peak_kilo_bytes = 0;
    let proc_path = format!("/proc/{pid}/statm");
    let sample_wait_duration = Duration::from_millis(sample_rate_ms as u64);
    let page_size = {
        let out = Command::new("getconf")
            .arg("PAGESIZE")
            .output()
            .expect("failed to run pmap");
        String::from_utf8_lossy(out.stdout.as_slice())
            .trim()
            .parse::<usize>()
            .expect("page size must be integer")
    };
    while receiver.try_recv().is_err() {
        let statm = std::fs::read_to_string(&proc_path).expect("could not read statm file");
        if let Some(pages) = statm.split_ascii_whitespace().nth(1) {
            if let Ok(pages) = pages.parse::<usize>() {
                peak_kilo_bytes = peak_kilo_bytes.max((pages * page_size) / 1000);
            }
        }

        std::thread::sleep(sample_wait_duration);
    }

    peak_kilo_bytes
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
#[clap(trailing_var_arg = true)]
struct Args {
    /// The number of milliseconds to wait between each sample
    #[arg(short, long)]
    sample_rate: Option<usize>,
    /// The number of milliseconds to wait before starting to collect samples
    #[arg(short, long)]
    timeout: Option<usize>,
    /// The program to run including arguments
    app: Vec<String>,
}

fn main() {
    let args = Args::parse();

    if args.app.is_empty() {
        eprintln!("no application given. Use --help for usage");
        exit(1);
    }

    let sample_rate_ms = args.sample_rate.unwrap_or(100);
    let timeout_ms = args.timeout.unwrap_or(0);
    let args = args.app;

    let (sender, receiver) = std::sync::mpsc::sync_channel::<()>(1);

    let binary_name = args[0].clone();
    let mut cmd = match Command::new(&binary_name)
        .args(args.into_iter().skip(1))
        .spawn()
    {
        Ok(cld) => cld,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!("binary \"{}\" not found: {e}", binary_name);
                }
                _ => eprintln!("error spawning child process: {e}"),
            };
            exit(1)
        }
    };
    let pid = cmd.id();

    let handle = std::thread::spawn(move || sample_loop(pid, sample_rate_ms, timeout_ms, receiver));

    cmd.wait().expect("could not wait for child");
    sender.send(()).expect("could not send message");
    let peak_kilo_bytes = handle.join().expect("error joining pmap thread");

    println!("Peak heap kilo bytes: {peak_kilo_bytes}");
}

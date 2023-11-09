use std::{
    process::{exit, Command},
    sync::mpsc::Receiver,
    time::Duration,
};

/// Continuously collects memory samples from a running process and returns the peak memory usage
/// in kilobytes.
///
/// # Arguments
///
/// * `binary_name` - The name of the binary being executed
/// * `pid` - The binary's pid
/// * `sample_rate_ms` - The sample rate in milliseconds, which is the time the sampling thread
/// waits until collecting another sample.
/// * `receiver` - A receiver which receives a message from the main thread when the child process
/// returns.
fn sample_loop(pid: u32, sample_rate_ms: usize, receiver: Receiver<()>) -> usize {
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

fn main() {
    let (sample_rate_ms, args) =
        if let Some(binary_arg_pos) = std::env::args().position(|s| s == "--") {
            if binary_arg_pos != 2 {
                eprintln!("invalid arguments. only sample rate is allowed before \"--\"");
                exit(1);
            }
            let args = std::env::args()
                .skip(binary_arg_pos + 1)
                .collect::<Vec<_>>();
            let sample_rate_ms = std::env::args()
                .nth(1)
                .map(|arg| {
                    arg.parse::<usize>()
                        .expect("sample rate must be a positive integer")
                })
                .unwrap();
            (sample_rate_ms, args)
        } else {
            (100usize, std::env::args().skip(1).collect::<Vec<_>>())
        };

    if args.is_empty() {
        eprintln!("Please enter a binary to run");
        exit(1);
    }

    let (sender, receiver) = std::sync::mpsc::sync_channel::<()>(1);

    let mut cmd = Command::new(args[0].clone())
        .args(args.into_iter().skip(1))
        .spawn()
        .expect("failed to start program");
    let pid = cmd.id();

    let handle = std::thread::spawn(move || sample_loop(pid, sample_rate_ms, receiver));

    cmd.wait().expect("could not wait for child");
    sender.send(()).expect("could not send message");
    let peak_kilo_bytes = handle.join().expect("error joining pmap thread");

    println!("Peak heap kilo bytes: {peak_kilo_bytes}");
}

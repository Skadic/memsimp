use std::{
    borrow::Cow,
    path::PathBuf,
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
fn sample_loop(
    binary_name: &str,
    pid: u32,
    sample_rate_ms: usize,
    receiver: Receiver<()>,
) -> usize {
    let mut peak_kilo_bytes = 0;
    let sample_wait_duration = Duration::from_millis(sample_rate_ms as u64);
    let allowed = [binary_name, "[ anon ]", "lib"];

    let pid_str = pid.to_string();
    while receiver.try_recv().is_err() {
        let cmd = Command::new("pmap")
            .arg(&pid_str)
            .output()
            .expect("failed to run pmap");

        let output: Cow<'_, str> = String::from_utf8_lossy(cmd.stdout.as_slice());
        // Parse the output of pmap and try collecting memory maps that should correspond to
        // allocated memory
        let sample_kilobytes = output
            .lines()
            .skip(1)
            .filter(|line| allowed.iter().any(|token| line.contains(token)))
            .filter_map(|line| line.split_ascii_whitespace().nth(1))
            .map(|kilobyte_str| &kilobyte_str[..kilobyte_str.len() - 1])
            .map(|kilobyte_str| {
                kilobyte_str
                    .parse::<usize>()
                    .expect("memory bites not integer")
            })
            .reduce(|l, r| l + r)
            .unwrap_or_default();

        peak_kilo_bytes = peak_kilo_bytes.max(sample_kilobytes);
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

    let os_binary_name = PathBuf::from(args[0].clone());
    let binary_name = os_binary_name
        .file_name()
        .expect("could not get file name")
        .to_string_lossy()
        .to_string();

    let mut cmd = Command::new(args[0].clone())
        .args(args.into_iter().skip(1))
        .spawn()
        .expect("failed to start program");
    let pid = cmd.id();

    let handle = std::thread::spawn(move || {
        sample_loop(binary_name.as_ref(), pid, sample_rate_ms, receiver)
    });

    cmd.wait().expect("could not wait for child");
    sender.send(()).expect("could not send message");
    let peak_kilo_bytes = handle.join().expect("error joining pmap thread");

    println!("Peak heap kilo bytes: {peak_kilo_bytes}");
}

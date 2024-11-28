use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

const DEFAULT_SLEEP_TIME: Duration = Duration::from_millis(250);
const FILE_CAP: usize = 4 << 10;
const TOTAL_CAP: usize = 32 << 10;

pub struct PollConfig {
    sleep_time: Duration,
}

#[derive(Serialize)]
struct PollHeader {
    files: Vec<String>,
    period: Duration,
}

fn create_header(files: &[PathBuf], cfg: &PollConfig) -> String {
    let header = PollHeader {
        files: files
            .iter()
            .cloned()
            .map(|p| p.to_str().unwrap().to_owned())
            .collect(),
        period: cfg.sleep_time,
    };
    let mut header = serde_json::to_string(&header).unwrap(); // should never fail
    header.push('\n'); // insert newline after the header
    header
}

fn store_header(output: &mut dyn Write, header: &str) {
    // dump and flush the poller header first to improve potential diagnostics
    output
        .write_all(header.as_bytes())
        .expect("failed to write header");
    output
        .flush()
        .expect("cannot flush the file after writing header");
}

pub fn poll_with_config(srcs: Vec<PathBuf>, dest: PathBuf, stop: Arc<AtomicBool>, cfg: PollConfig) {
    // open destination file with the final content and store header
    let mut output = File::create(dest).expect("cannot open file");
    store_header(&mut output, &create_header(&srcs, &cfg));

    let mut strbuffer = String::with_capacity(FILE_CAP);
    let mut outbuffer = String::with_capacity(TOTAL_CAP);

    while !stop.load(Ordering::Acquire) {
        // clear the previous content
        outbuffer.clear();

        // prepare the common timestamp
        let now = chrono::Local::now();
        outbuffer.push_str(&now.to_rfc3339_opts(chrono::SecondsFormat::Micros, false));
        outbuffer.push('\n');

        // read the files
        for src in &srcs {
            // read the file content
            strbuffer.clear();
            File::open(src)
                .and_then(|mut f| f.read_to_string(&mut strbuffer))
                .expect("cannot open/read file");

            outbuffer.push_str(&strbuffer);
        }

        // add the final delimiter and flush the output
        outbuffer.push('\n');
        output
            .write_all(outbuffer.as_bytes())
            .expect("cannot write");

        std::thread::sleep(cfg.sleep_time);
    }

    output.flush().expect("cannot flush");
}

pub fn poll(srcs: Vec<PathBuf>, dest: PathBuf, stop: Arc<AtomicBool>) {
    poll_with_config(
        srcs,
        dest,
        stop,
        PollConfig {
            sleep_time: DEFAULT_SLEEP_TIME,
        },
    )
}

#[test]
fn single_file_poll() {
    let stop: Arc<AtomicBool> = Arc::default();
    let stop2 = stop.clone();

    let poller = std::thread::spawn(move || {
        poll(
            vec![PathBuf::from("/proc/meminfo")],
            PathBuf::from("output_single"),
            stop,
        )
    });

    std::thread::sleep(std::time::Duration::from_secs(3));
    stop2.store(true, std::sync::atomic::Ordering::Release);
    poller.join().unwrap();
}

#[test]
fn multiple_file_poll() {
    let stop: Arc<AtomicBool> = Arc::default();
    let stop2 = stop.clone();

    let poller = std::thread::spawn(move || {
        poll(
            vec![PathBuf::from("/proc/1/stat"), PathBuf::from("/proc/2/stat")],
            PathBuf::from("output_multifile"),
            stop,
        )
    });

    std::thread::sleep(std::time::Duration::from_secs(3));
    stop2.store(true, std::sync::atomic::Ordering::Release);
    poller.join().unwrap();
}

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const SLEEP_TIME: Duration = Duration::from_millis(250);
const FILE_CAP: usize = 4 << 10;
const TOTAL_CAP: usize = 64 << 10;

pub fn poll(srcs: Vec<PathBuf>, dest: PathBuf, stop: Arc<AtomicBool>) {
    // open destination file with the final content
    let mut output = File::create(dest).expect("cannot open file");

    let mut strbuffer = String::new();
    let mut outbuffer = String::new();
    strbuffer.reserve(FILE_CAP);
    outbuffer.reserve(TOTAL_CAP);

    while !stop.load(Ordering::Acquire) {
        // clear the previous content
        outbuffer.clear();

        // prepare the common timestamp
        outbuffer
            .push_str(&chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false));
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

        std::thread::sleep(SLEEP_TIME);
    }

    output.flush().expect("cannot flush");
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

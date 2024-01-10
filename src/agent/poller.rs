use std::fs::File;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const SLEEP_TIME: Duration = Duration::from_millis(250);

pub fn poll(path: String, dest: String, stop: Arc<AtomicBool>) {
    // open destination file with the final content
    let mut output = File::create(dest).expect("cannot open file");

    let mut strbuffer = String::new();
    strbuffer.reserve(8192);

    while !stop.load(Ordering::Acquire) {
        // clear the previous content
        strbuffer.clear();

        // read the file content
        File::open(&path)
            .and_then(|mut f| f.read_to_string(&mut strbuffer))
            .expect("cannot open/read file");

        // prepare the string
        let result = format!(
            "{}\n{}\n",
            chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false),
            strbuffer
        );
        output.write_all(result.as_bytes()).expect("cannot write");

        std::thread::sleep(SLEEP_TIME);
    }

    output.flush().expect("cannot flush");
}

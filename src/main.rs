use std::path::{Path, PathBuf};

use env_logger::Env;
use log::{error, info};

mod agent;
mod protocol_impl;

/// Little helper function to convert str literals to error message.
fn emsg<T, U: ?Sized + AsRef<str>>(s: &U) -> Result<T, String> {
    Err(s.as_ref().into())
}

fn find_max_numeric_dir(base: &Path) -> u32 {
    let mut max_dir = 0;

    for dir in base.read_dir().expect("cannot read dir").flatten() {
        let name = dir.file_name();
        match name.to_string_lossy().parse::<u32>() {
            Ok(value) => max_dir = std::cmp::max(max_dir, value),
            Err(_) => continue,
        }
    }

    max_dir
}

fn create_outdir(base: PathBuf) -> Result<PathBuf, String> {
    if base.exists() && !base.is_dir() {
        return emsg(&format!(
            "path provided '{}' is not a directory",
            base.to_string_lossy()
        ));
    }

    let new_dir_num = if base.exists() {
        find_max_numeric_dir(&base) + 1
    } else {
        0
    };

    let new_dir = base.join(Path::new(&new_dir_num.to_string()));
    std::fs::create_dir_all(&new_dir).unwrap_or_else(|_| panic!("cannot create dir {:?}", new_dir));

    Ok(new_dir)
}

fn main_local(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return emsg("usage: PROG local PATH_TO_CONFIG PATH_TO_OUTPUT");
    }

    let json_path = &args[0];
    let logs_path = PathBuf::from(&args[1]);
    let outdir = create_outdir(logs_path)?;

    info!("agent is in local mode with config: {}", json_path);
    info!("output directory: {}", outdir.to_string_lossy());
    let proto = protocol_impl::LocalProtocol::from_json(json_path)?;
    let agent = agent::Agent::new(proto, outdir.clone());

    info!("staring the agent");
    agent.serve();

    info!("done, output directory: {}", outdir.to_string_lossy());
    Ok(())
}

fn main_tcp(_args: &[String]) -> Result<(), String> {
    emsg("tcp transport not implemented")
}

fn main_wrapper(args: &[String]) -> Result<(), String> {
    // init log with Info level by default
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("pmppt-agent");

    if args.len() < 2 {
        return emsg("usage: PROG (tcp|local) ARGS...");
    }

    match args[1].as_str() {
        "local" => main_local(&args[2..]),
        "tcp" => main_tcp(&args[2..]),
        _ => emsg("Only 'tcp' or 'local' transports supported"),
    }
}

fn main() {
    // TODO: here will be better CLI arguments parsing
    let args: Vec<String> = std::env::args().collect();
    if let Err(msg) = main_wrapper(&args) {
        error!("Error: {}", msg);
        std::process::exit(1);
    }
}

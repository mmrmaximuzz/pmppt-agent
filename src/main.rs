use env_logger::Env;
use log::{error, info};

mod agent;
mod protocol_impl;

/// Little helper function to convert str literals to error message.
fn emsg(s: &str) -> Result<(), String> {
    Err(s.into())
}

fn main_local(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return emsg("usage: PROG local PATH_TO_CONFIG");
    }

    let json_path = &args[0];
    info!("starting agent in local mode with config: {}", json_path);

    let proto = protocol_impl::LocalProtocol::from_json(json_path)?;
    let agent = agent::Agent::new(proto);
    agent.serve();
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

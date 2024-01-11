mod agent;
mod protocol_impl;

fn main_local(_args: &[String]) {
    unimplemented!()
}

fn main_tcp(_args: &[String]) {
    unimplemented!()
}

fn main() {
    // TODO: here will be better CLI arguments parsing
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} (tcp|local) ARGS...", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "local" => return main_local(&args[2..]),
        "tcp" => return main_tcp(&args[2..]),
        _ => {
            eprintln!("Only 'tcp' or 'local' transports supported");
            std::process::exit(1);
        }
    }
}

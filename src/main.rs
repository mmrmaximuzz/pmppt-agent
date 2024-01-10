use std::net::TcpListener;

mod agent;

fn main() {
    // TODO: here will be better CLI arguments parsing
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} TCP_PORT", args[0]);
        return;
    }

    let port: u16 = args[1].parse().expect("must be a correct TCP port value");
    let endpoint = format!("0.0.0.0:{}", port);
    println!("pmppt-agent. Listening on {}", endpoint);

    let server = TcpListener::bind(endpoint).expect("cannot create TCP sock");
    loop {
        let (connection, remote) = server.accept().expect("failed to accept client");
        println!("Got new connection from {}", remote);

        agent::Agent::new(connection).serve();
        println!("Done with connection from {}", remote);
    }
}

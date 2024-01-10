use std::io::Read;
use std::net::TcpStream;

mod poller;

pub struct Agent {
    iostream: TcpStream,
}

impl Agent {
    pub fn new(iostream: TcpStream) -> Self {
        Self {
            iostream,
        }
    }

    pub fn serve(mut self) {
        loop {
            match recv_message(&mut self.iostream) {
                Some(msg) => self.handle_message(msg),
                None => {
                    eprintln!("Error: incorrect message, stop serving");
                    break;
                },
            }
        }

        // stop itself before Drop
        self.stop();
    }

    fn handle_message(&mut self, msg: Vec<u8>) {
        println!("Handling message: {:?}", msg);
    }

    fn stop(self) {
        println!("Releasing all the resources");
    }
}

fn recv_message(iostream: &mut TcpStream) -> Option<Vec<u8>> {
    // read the header (msg length) first
    let mut bytes = [0; 2];
    iostream.read_exact(&mut bytes).ok()?;

    // read the message itself
    let msg_len = u16::from_le_bytes(bytes) as usize;
    let mut message = vec![0; msg_len];
    iostream.read_exact(&mut message).ok()?;

    Some(message)
}

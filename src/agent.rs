mod poller;
pub mod protocol;

use protocol::{PmpptRequest, Protocol};

pub struct Agent<P: Protocol> {
    proto: P,
}

impl<P> Agent<P>
where
    P: Protocol,
{
    pub fn new(proto: P) -> Self {
        Self { proto }
    }

    pub fn serve(mut self) {
        loop {
            match self.proto.recv_request() {
                None => {
                    eprintln!("Error: incorrect message, stop serving");
                    break;
                }
                Some(PmpptRequest::Finish {}) => {
                    println!("Got finish request, stopping running activities");
                    break;
                }
                Some(msg) => self.handle_message(msg),
            }
        }

        // stop itself before Drop
        self.stop();
    }

    fn handle_message(&mut self, _msg: PmpptRequest) {
        println!("Handling message");
    }

    fn stop(self) {
        println!("Releasing all the resources");
    }
}

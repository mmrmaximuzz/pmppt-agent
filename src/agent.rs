use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

use log::{error, info};

mod poller;
pub mod protocol;
use protocol::{PmpptRequest, Protocol};

/// PMPPT Agent instance.
///
/// This structure is generic over [`Protocol`] trait, allowing different implementation of message
/// transport between pmppt-agent and its controllers. Agent communicates with its controllers and
/// executes performance measurement scenario, keeping all allocated resources inside this
/// structure.
pub struct Agent<P: Protocol> {
    proto: P,
    count: u32,
    polls: HashMap<u32, Poll>,
}

struct Poll {
    stop: Arc<AtomicBool>,
    thrd: JoinHandle<()>,
}

impl<P> Agent<P>
where
    P: Protocol,
{
    pub fn new(proto: P) -> Self {
        Self {
            proto,
            count: 0,
            polls: HashMap::default(),
        }
    }

    pub fn serve(mut self) {
        info!("agent started");

        loop {
            match self.proto.recv_request() {
                None => {
                    error!("got incorrect message, stop serving");
                    break;
                }
                Some(PmpptRequest::Finish {}) => {
                    info!("got 'finish' request, stopping running activities");
                    break;
                }
                Some(msg) => self.handle_message(msg),
            }
        }

        // stop itself before Drop
        self.stop();
    }

    fn spawn_poller(&mut self, path: String) {
        let stop_flag_agent = Arc::new(AtomicBool::default());
        let stop_flag_thread = stop_flag_agent.clone();
        let poll_id = self.count;

        let poll_thread = std::thread::spawn(move || {
            poller::poll(path, format!("{}.log", poll_id), stop_flag_thread)
        });

        let res = self.polls.insert(
            poll_id,
            Poll {
                stop: stop_flag_agent,
                thrd: poll_thread,
            },
        );
        assert!(res.is_none(), "got duplicate pollers on {}", poll_id);

        self.count += 1;
    }

    fn handle_message(&mut self, msg: PmpptRequest) {
        match msg {
            PmpptRequest::Poll { path } => {
                info!("starting to poll: {}", path);
                self.spawn_poller(path);
            }
            PmpptRequest::Finish {} => unreachable!("Finish message is already processed outside"),
        }
    }

    fn stop(mut self) {
        info!("stopping agent");

        // first set stop bits to all threads, then join to allow threads to stop in parallel
        for poll in self.polls.values() {
            poll.stop.store(true, std::sync::atomic::Ordering::Release);
        }

        for (id, poll) in self.polls.drain() {
            poll.thrd
                .join()
                .unwrap_or_else(|_| panic!("cannot join polling thread: {}", id));
        }
    }
}

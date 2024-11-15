use std::{
    collections::HashMap,
    fs::File,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

use log::{error, info};
use subprocess::{Exec, Popen};

mod poller;
pub mod protocol;
use protocol::{PmpptRequest, Protocol, SpawnMode};

/// PMPPT Agent instance.
///
/// This structure is generic over [`Protocol`] trait, allowing different implementation of message
/// transport between pmppt-agent and its controllers. Agent communicates with its controllers and
/// executes performance measurement scenario, keeping all allocated resources inside this
/// structure.
pub struct Agent<P: Protocol> {
    proto: P,
    count: u32,
    outdir: PathBuf,
    polls: HashMap<u32, Poll>,
    procs: HashMap<u32, Proc>,
}

struct Poll {
    stop: Arc<AtomicBool>,
    thrd: JoinHandle<()>,
    name: String,
}

struct Proc {
    popen: Popen,
    wait4: bool,
    name: String,
}

impl<P> Agent<P>
where
    P: Protocol,
{
    pub fn new(proto: P, outdir: PathBuf) -> Self {
        Self {
            proto,
            count: 0,
            outdir,
            polls: HashMap::default(),
            procs: HashMap::default(),
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

    fn get_next_id(&mut self) -> u32 {
        let id = self.count;
        self.count += 1;
        id
    }

    fn spawn_poller(&mut self, paths: &[PathBuf], name: &str) {
        let id = self.get_next_id();
        let path_out = self.outdir.join(format!("{:03}-poll.log", id));
        let paths = paths.to_owned(); // full clone to send to thread

        let stop_flag_agent = Arc::new(AtomicBool::default());
        let stop_flag_thread = stop_flag_agent.clone();
        let poll_thread =
            std::thread::spawn(move || poller::poll(paths, path_out, stop_flag_thread));

        let res = self.polls.insert(
            id,
            Poll {
                stop: stop_flag_agent,
                thrd: poll_thread,
                name: name.to_owned(),
            },
        );
        assert!(res.is_none(), "got duplicate poll/proc on {}", id);

        info!("Poller:   id={}, path='{}'", id, name);
    }

    fn spawn_process_foreground(&mut self, cmd: String, args: Vec<String>) {
        let id = self.get_next_id();
        let file_out = File::create_new(self.outdir.join(format!("{:03}-out.log", id))).unwrap();
        let file_err = File::create_new(self.outdir.join(format!("{:03}-err.log", id))).unwrap();

        let cmd = Exec::cmd(&cmd)
            .args(&args)
            .stdout(file_out)
            .stderr(file_err);

        // collect the name before spawning the process
        let name = cmd.to_cmdline_lossy();
        let status = cmd.join().expect("failed to capture output");

        info!("FG spawn: id={}, name='{}', success={:?}", id, name, status);
    }

    fn spawn_process_background(&mut self, cmd: String, args: Vec<String>, wait4: bool) {
        let id = self.get_next_id();
        let file_out = File::create_new(self.outdir.join(format!("{:03}-out.log", id))).unwrap();
        let file_err = File::create_new(self.outdir.join(format!("{:03}-err.log", id))).unwrap();

        let cmd = Exec::cmd(&cmd)
            .args(&args)
            .stdout(file_out)
            .stderr(file_err);

        let name = cmd.to_cmdline_lossy();
        let popen = cmd.popen().expect("failed to start process");

        let res = self.procs.insert(
            id,
            Proc {
                popen,
                wait4,
                name: name.clone(),
            },
        );
        assert!(res.is_none(), "got duplicate poll/proc on {}", id);

        info!("BG spawn: id={}, name='{}', wait4={}", id, name, wait4);
    }

    fn spawn_process(&mut self, cmd: String, args: Vec<String>, mode: SpawnMode) {
        match mode {
            SpawnMode::Foreground => self.spawn_process_foreground(cmd, args),
            SpawnMode::BackgroundWait => self.spawn_process_background(cmd, args, true),
            SpawnMode::BackgroundKill => self.spawn_process_background(cmd, args, false),
        }
    }

    fn handle_message(&mut self, msg: PmpptRequest) {
        match msg {
            PmpptRequest::Poll { path } => {
                self.spawn_poller(&[PathBuf::from(&path)], &path);
            }
            PmpptRequest::PollGlob { glob: pattern } => {
                let paths: Vec<PathBuf> = glob::glob(&pattern)
                    .expect("failed to lookup glob pattern")
                    .map(|g| g.unwrap())
                    .collect();
                self.spawn_poller(&paths, &pattern);
            }
            PmpptRequest::Spawn { cmd, args, mode } => {
                self.spawn_process(cmd, args, mode);
            }
            PmpptRequest::Finish {} => unreachable!("Finish message is already processed outside"),
        }
    }

    fn stop(mut self) {
        info!("stopping agent");

        // stop in reverse order
        for i in (0..self.count).rev() {
            match (self.procs.remove(&i), self.polls.remove(&i)) {
                (Some(mut proc), None) => {
                    info!("stopping process id={}, name='{}'", i, proc.name);
                    if !proc.wait4 {
                        // send the signal to terminate it now
                        proc.popen
                            .terminate()
                            .unwrap_or_else(|_| panic!("failed to terminate process {}", i));
                    }

                    proc.popen
                        .wait()
                        .unwrap_or_else(|_| panic!("failed to wait for the process {}", i));
                }

                (None, Some(poll)) => {
                    info!("stopping poller  id={}, name='{}'", i, poll.name);
                    poll.stop.store(true, std::sync::atomic::Ordering::Release);
                    poll.thrd
                        .join()
                        .unwrap_or_else(|_| panic!("cannot join polling thread: {}", i));
                }

                // OK, it was FG process
                (None, None) => (),

                // this should never happen
                _ => unreachable!("found both process and poller for id={}", i),
            }
        }
    }
}

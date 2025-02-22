use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

use log::{error, info, warn};
use subprocess::{Exec, Popen};

mod poller;
pub mod protocol;
use protocol::{IdOrError, OutOrError, PmpptRequest, PmpptResponse, Protocol, SpawnMode};

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

        let is_abnormal = loop {
            match self.proto.recv_request() {
                None => {
                    error!("failed to get correct message, stop serving agent");
                    break true;
                }
                Some(PmpptRequest::Abort) => {
                    warn!("got 'abort' request, emergency stop");
                    break true;
                }
                Some(PmpptRequest::Finish) => {
                    info!("got 'finish' request, stopping running activities");
                    break false;
                }
                Some(msg) => self.handle_message(msg),
            }
        };

        // stop itself before Drop
        self.stop(is_abnormal);
    }

    fn get_next_id(&mut self) -> u32 {
        self.count += 1;
        self.count
    }

    fn spawn_poller(&mut self, paths: &[PathBuf], name: &str) -> IdOrError {
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

        // TODO: add checks for failures in poller spawning
        Ok(id)
    }

    fn spawn_process_foreground(&mut self, cmd: String, args: Vec<String>) -> OutOrError {
        let id = self.get_next_id();
        let outpath = self.outdir.join(format!("{:03}-out.log", id));
        let errpath = self.outdir.join(format!("{:03}-err.log", id));
        let file_out = File::create_new(outpath.clone()).unwrap();
        let file_err = File::create_new(errpath.clone()).unwrap();

        let cmd = Exec::cmd(&cmd)
            .args(&args)
            .stdout(file_out)
            .stderr(file_err);

        // collect the name before spawning the process
        let name = cmd.to_cmdline_lossy();

        info!("FG spawn: id={}, name='{}'", id, name);

        let status = cmd.join().map_err(|e| {
            let msg = format!("failed to spawn fg process: {}", e);
            error!("{}", msg);
            msg
        })?;

        info!("FG spawn: id={}, name='{}', success={:?}", id, name, status);

        // collect the results
        let mut stdout = Vec::with_capacity(4096);
        let mut stderr = Vec::with_capacity(4096);
        File::open(outpath)
            .unwrap()
            .read_to_end(&mut stdout)
            .expect("cannot read stdout file");
        File::open(errpath)
            .unwrap()
            .read_to_end(&mut stderr)
            .expect("cannot read stderr file");

        Ok((stdout, stderr))
    }

    fn spawn_process_background(
        &mut self,
        cmd: String,
        args: Vec<String>,
        wait4: bool,
    ) -> IdOrError {
        let id = self.get_next_id();
        let file_out = File::create_new(self.outdir.join(format!("{:03}-out.log", id))).unwrap();
        let file_err = File::create_new(self.outdir.join(format!("{:03}-err.log", id))).unwrap();

        let cmd = Exec::cmd(&cmd)
            .args(&args)
            .stdout(file_out)
            .stderr(file_err);

        let name = cmd.to_cmdline_lossy();
        let popen = cmd.popen().map_err(|e| {
            let msg = format!("failed to spawn bg process: {}", e);
            error!("{}", msg);
            msg
        })?;

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

        Ok(id)
    }

    fn spawn_process(&mut self, cmd: String, args: Vec<String>, mode: SpawnMode) -> PmpptResponse {
        match mode {
            SpawnMode::Foreground => {
                PmpptResponse::SpawnFg(self.spawn_process_foreground(cmd, args))
            }
            SpawnMode::BackgroundWait => {
                PmpptResponse::SpawnBg(self.spawn_process_background(cmd, args, true))
            }
            SpawnMode::BackgroundKill => {
                PmpptResponse::SpawnBg(self.spawn_process_background(cmd, args, false))
            }
        }
    }

    fn handle_message(&mut self, msg: PmpptRequest) {
        match msg {
            PmpptRequest::Poll { pattern } => {
                // expand braces and interpret each expansion as a glob
                let paths: Vec<PathBuf> = brace_expand::brace_expand(&pattern)
                    .into_iter()
                    .flat_map(|p| {
                        glob::glob(&p)
                            .expect("failed to lookup glob pattern")
                            .map(|g| g.unwrap())
                    })
                    .collect();

                // TODO: fail even if just a single brace expansion led to nothing
                // interpret empty search result as a failure
                let res = if !paths.is_empty() {
                    self.spawn_poller(&paths, &pattern)
                } else {
                    let msg = format!("got empty search result on expanding '{}'", pattern);
                    error!("{}", msg);
                    Err(msg)
                };

                self.proto.send_response(PmpptResponse::Poll(res));
            }
            PmpptRequest::Spawn { cmd, args, mode } => {
                let res = self.spawn_process(cmd, args, mode);
                self.proto.send_response(res);
            }
            PmpptRequest::Finish => unreachable!("Finish must be already processed outside"),
            PmpptRequest::Abort => unreachable!("Abort must be already processed outside"),
        }
    }

    fn stop(mut self, abnormal: bool) {
        let mode = if abnormal { "emergency" } else { "graceful" };
        info!("stopping agent in {} mode", mode);

        // stop in reverse order
        for i in (1..=self.count).rev() {
            match (self.procs.remove(&i), self.polls.remove(&i)) {
                (Some(mut proc), None) => {
                    info!("stopping process id={}, name='{}'", i, proc.name);
                    if !proc.wait4 || abnormal {
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

                // OK, it was FG process or it has been stopped already by the pmppt client
                (None, None) => (),

                // this should never happen
                _ => unreachable!("found both process and poller for id={}", i),
            }
        }

        // sanity checks
        assert!(self.polls.is_empty());
        assert!(self.procs.is_empty());
    }
}

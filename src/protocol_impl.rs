//! Implementations of PMPPT protocol for the agent.

use std::fs;
use std::io::Read;
use std::time::Duration;

use log::{debug, error};
use serde::Deserialize;
use serde_json::Value;

use crate::agent::protocol::{PmpptRequest, PmpptResponse, Protocol, SpawnMode};

#[derive(Deserialize)]
#[allow(non_camel_case_types)]
enum ExecMode {
    fg,
    bgwait,
    bgkill,
}

fn local_mode_to_agent(mode: Option<ExecMode>) -> SpawnMode {
    match mode {
        // default spawn is foreground
        None => SpawnMode::Foreground,
        // others are just mapped
        Some(ExecMode::fg) => SpawnMode::Foreground,
        Some(ExecMode::bgwait) => SpawnMode::BackgroundWait,
        Some(ExecMode::bgkill) => SpawnMode::BackgroundKill,
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
enum LocalRequest {
    // mapped PMPPT commands
    Poll {
        pattern: String,
    },
    Spawn {
        cmd: String,
        args: Option<Vec<String>>,
        mode: Option<ExecMode>,
    },
    Abort,
    // local transport commands (non-PMPPT)
    Pause {
        prompt: Option<String>,
    },
    Sleep {
        time: f64,
    },
}

pub struct LocalProtocol {
    requests: Vec<LocalRequest>,
    current: Option<PmpptRequest>,
}

impl LocalProtocol {
    pub fn from_json(json_path: &str) -> Result<Self, String> {
        // first read the JSON file completely
        let content = fs::read_to_string(json_path)
            .map_err(|e| format!("cannot read '{}' - {}", json_path, e))?;

        // parse as raw JSON list first
        let values: Vec<Value> =
            serde_json::from_str(&content).map_err(|e| format!("bad JSON format - {}", e))?;

        // then map every command to PMPPT protocol
        let mut requests: Vec<LocalRequest> = serde_json::from_value(Value::Array(values))
            .map_err(|e| format!("unsupported command found: {}", e))?;

        // reverse the vector to extract the elements with `pop`
        requests.reverse();

        Ok(LocalProtocol {
            requests,
            current: None,
        })
    }
}

const GENERIC_PROMPT: &str = r#"
==================================================
=======   Further execution is paused.     =======
======= Press Enter to continue execution. =======
==================================================
"#;

impl Protocol for LocalProtocol {
    fn recv_request(&mut self) -> Option<PmpptRequest> {
        // Extract the new local agent request from the config.
        //
        // In local mode we don't have any real PMPPT controller connected. So here we try to
        // imitate its existence by remembering the current executing request to associate agent
        // responses with it.
        self.current = loop {
            match self.requests.pop() {
                Some(local_req) => match local_req {
                    // provide mapped command as-is
                    LocalRequest::Poll { pattern } => break PmpptRequest::Poll { pattern },
                    LocalRequest::Spawn { cmd, args, mode } => {
                        break PmpptRequest::Spawn {
                            cmd,
                            args: args.unwrap_or_default(), // default is no args
                            mode: local_mode_to_agent(mode), // default is foreground
                        };
                    }
                    LocalRequest::Abort => break PmpptRequest::Abort,

                    // handle local commands specially
                    LocalRequest::Sleep { time } => {
                        std::thread::sleep(Duration::from_secs_f64(time));
                        continue;
                    }
                    LocalRequest::Pause { prompt } => {
                        println!("{}", GENERIC_PROMPT.trim());
                        if let Some(prompt) = prompt {
                            println!("Description: {}", prompt);
                        }
                        std::io::stdin()
                            .read_exact(&mut [0u8])
                            .expect("stdin is broken");
                    }
                },

                // when local requests are over, implicitly generate Finish request
                None => break PmpptRequest::Finish,
            }
        }
        .into();

        // return the request to the agent to execute
        self.current.clone()
    }

    // imitate that we "receive" a response from PMPPT agent
    fn send_response(&mut self, response: PmpptResponse) -> Option<()> {
        match response {
            // TODO: stop the execution instead of just panic
            PmpptResponse::Poll(Err(msg)) => {
                error!(
                    r#"Poll request failed: req={:?}, error="{}""#,
                    self.current, msg
                );

                // emulate the Abort message from the controller
                self.requests.push(LocalRequest::Abort);
            }

            PmpptResponse::Poll(Ok(id)) => {
                debug!("Poll result: id={}", id);
            }
        }

        // in local mode this function cannot fail
        Some(())
    }
}

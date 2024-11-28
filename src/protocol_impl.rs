//! Implementations of PMPPT protocol for the agent.

use std::fs;
use std::io::Read;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::agent::protocol;
use crate::agent::protocol::{PmpptRequest, SpawnMode};

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

        Ok(LocalProtocol { requests })
    }
}

const GENERIC_PROMPT: &str = r#"
==================================================
=======   Further execution is paused.     =======
======= Press Enter to continue execution. =======
==================================================
"#;

impl protocol::Protocol for LocalProtocol {
    fn recv_request(&mut self) -> Option<PmpptRequest> {
        loop {
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
                // when local requests are over, implicitly send Finish command
                None => break PmpptRequest::Finish {},
            }
        }
        .into()
    }

    fn send_response(&mut self, _response: protocol::PmpptResponse) -> Option<()> {
        todo!()
    }
}

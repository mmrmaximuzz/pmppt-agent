//! Implementations of PMPPT protocol for the agent.

use std::fs;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::agent::protocol;
use crate::agent::protocol::PmpptRequest;

#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
enum LocalRequest {
    // mapped PMPPT commands
    Poll { path: String },
    // local transport commands (non-PMPPT)
    Sleep { time: f64 },
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

impl protocol::Protocol for LocalProtocol {
    fn recv_request(&mut self) -> Option<protocol::PmpptRequest> {
        loop {
            match self.requests.pop() {
                Some(local_req) => match local_req {
                    // provide mapped command as-is
                    LocalRequest::Poll { path } => break PmpptRequest::Poll { path },
                    // handle local commands specially
                    LocalRequest::Sleep { time } => {
                        std::thread::sleep(Duration::from_secs_f64(time));
                        continue;
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

//! Implementations of PMPPT protocol for the agent.

use std::fs;

use serde::Deserialize;
use serde_json::Value;

use crate::agent::protocol;

#[derive(Deserialize)]
#[serde(tag = "type", content = "data")]
enum LocalRequest {
    Poll { path: String },
    Sleep { time: f64 },
    Stop {},
}

pub struct LocalProtocol {
    requests: Vec<protocol::PmpptRequest>,
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
        let mut requests = vec![];
        for value in values {
            let request = match serde_json::from_value(value.clone())
                .map_err(|_| format!("bad/unsupported request {}", value))?
            {
                LocalRequest::Poll { path } => protocol::PmpptRequest::Poll { path },
                LocalRequest::Stop {} => protocol::PmpptRequest::Finish {},
                LocalRequest::Sleep { time } => protocol::PmpptRequest::Sleep { time },
            };
            requests.push(request);
        }

        // reverse to allow consuming commands with .pop() call
        requests.reverse();

        Ok(LocalProtocol { requests })
    }
}

impl protocol::Protocol for LocalProtocol {
    fn recv_request(&mut self) -> Option<protocol::PmpptRequest> {
        self.requests.pop()
    }

    fn send_response(&mut self, _response: protocol::PmpptResponse) -> Option<()> {
        todo!()
    }
}

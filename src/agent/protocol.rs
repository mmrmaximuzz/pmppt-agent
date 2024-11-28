//! Module defining PMPPT protocol between host and agent.

/// Input data for the agent.
pub enum PmpptRequest {
    Poll {
        pattern: String,
    },
    Spawn {
        cmd: String,
        args: Vec<String>,
        mode: SpawnMode,
    },
    Finish {},
}

#[derive(Debug)]
pub enum SpawnMode {
    Foreground,
    BackgroundWait,
    BackgroundKill,
}

/// Agent's responses.
pub enum PmpptResponse {
    Poll { id: u32 },
}

/// Generic transport protocol interface.
pub trait Protocol {
    fn recv_request(&mut self) -> Option<PmpptRequest>;
    fn send_response(&mut self, response: PmpptResponse) -> Option<()>;
}

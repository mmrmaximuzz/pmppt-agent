//! Module defining PMPPT protocol between host and agent.

/// Input data for the agent.
#[derive(Debug, Clone)]
pub enum PmpptRequest {
    Poll {
        pattern: String,
    },
    Spawn {
        cmd: String,
        args: Vec<String>,
        mode: SpawnMode,
    },
    Finish,
    Abort,
}

#[derive(Debug, Clone, Copy)]
pub enum SpawnMode {
    Foreground,
    BackgroundWait,
    BackgroundKill,
}

pub type IdOrError = Result<u32, String>;
pub type OutOrError = Result<(Vec<u8>, Vec<u8>), String>;

/// Agent's responses.
pub enum PmpptResponse {
    Poll(IdOrError),
    SpawnFg(OutOrError),
    SpawnBg(IdOrError),
}

/// Generic transport protocol interface.
pub trait Protocol {
    fn recv_request(&mut self) -> Option<PmpptRequest>;
    fn send_response(&mut self, response: PmpptResponse) -> Option<()>;
}

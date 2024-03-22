//! Module defining PMPPT protocol between host and agent.

/// Input data for the agent.
pub enum PmpptRequest {
    Poll { path: String },
    Sleep { time: f64 },
    Finish {},
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

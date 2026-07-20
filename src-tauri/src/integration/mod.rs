//! Authenticated loopback API for the Chromium extension.

mod auth;
mod handlers;
mod server;
mod types;

pub use server::{build_router, serve, LOOPBACK_ADDR, LOOPBACK_PORT};
pub use types::{AddProblemRequest, PairRequest, PendingCompletionDto};

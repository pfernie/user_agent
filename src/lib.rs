#[cfg(test)]
use env_logger;
use failure;

#[macro_use]
mod session;
mod reqwest_session;
mod utils;
pub use crate::reqwest_session::ReqwestSession;
pub use crate::session::{Session, SessionClient, SessionRequest, SessionResponse};

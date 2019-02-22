use failure;

#[macro_use]
mod session;
mod reqwest_session;
mod utils;
pub use crate::reqwest_session::{ReqwestSession, ReqwestSessionError};
pub use crate::session::{Session, SessionClient, SessionRequest, SessionResponse};
pub use cookie_store::CookieError;

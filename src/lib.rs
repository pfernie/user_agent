#[cfg(test)]
use env_logger;
use failure::Fail;
use idna;
use log;
use publicsuffix;
use reqwest;
use serde;
use time;
use try_from;
use url;

mod cookie;
pub use crate::cookie::Error as CookieError;
pub use crate::cookie::{Cookie, CookieResult};
mod cookie_domain;
mod cookie_expiration;
mod cookie_path;
mod cookie_store;
pub use crate::cookie_store::CookieStore;
#[macro_use]
mod session;
pub use crate::session::{Session, WithSession};
pub mod reqwest_session;
pub use crate::reqwest_session::ReqwestSession;
mod utils;

#[derive(Debug, Fail)]
#[fail(display = "IDNA errors: {:#?}", _0)]
pub struct IdnaErrors(idna::uts46::Errors);

impl From<idna::uts46::Errors> for IdnaErrors {
    fn from(e: idna::uts46::Errors) -> Self {
        IdnaErrors(e)
    }
}

#[derive(Debug, Fail)]
pub enum ReqwestSessionError {
    #[fail(display = "URL parse error: {}", _0)]
    ParseUrlError(url::ParseError),
    #[fail(display = "Reqwest error: {}", _0)]
    ReqwestError(reqwest::Error),
}

impl From<url::ParseError> for ReqwestSessionError {
    fn from(e: url::ParseError) -> Self {
        ReqwestSessionError::ParseUrlError(e)
    }
}

impl From<reqwest::Error> for ReqwestSessionError {
    fn from(e: reqwest::Error) -> Self {
        ReqwestSessionError::ReqwestError(e)
    }
}

pub type Result<T> = std::result::Result<T, failure::Error>;

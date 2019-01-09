extern crate cookie as raw_cookie;
#[cfg(test)]
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate idna;
#[macro_use]
extern crate log;
extern crate publicsuffix;
extern crate serde;
#[cfg_attr(test, macro_use)]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate reqwest;
extern crate time;
extern crate try_from;
extern crate url;

mod cookie;
pub use cookie::Error as CookieError;
pub use cookie::{Cookie, CookieResult};
mod cookie_domain;
mod cookie_expiration;
mod cookie_path;
mod cookie_store;
pub use cookie_store::CookieStore;
#[macro_use]
mod session;
pub use session::{Session, WithSession};
pub mod reqwest_session;
pub use reqwest_session::ReqwestSession;
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

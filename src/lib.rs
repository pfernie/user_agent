extern crate cookie as raw_cookie;
#[macro_use]
extern crate derive_error_chain;
extern crate error_chain;
extern crate env_logger;
extern crate idna;
#[macro_use]
extern crate log;
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
pub use cookie::{Cookie, CookieResult};
pub use cookie::Error as CookieError;
mod cookie_domain;
mod cookie_expiration;
mod cookie_path;
mod cookie_store;
pub use cookie_store::CookieStore;
#[macro_use]
mod session;
pub use session::{Session, WithSession};
mod reqwest_session;
pub use reqwest_session::ReqwestSession;
mod utils;

use idna::uts46::Errors as IdnaError;
#[derive(Debug, error_chain)]
pub enum ErrorKind {
    Msg(String),
    /// IO Error
    #[error_chain(foreign)]
    Io(std::io::Error),
    /// IDNA Parse Error
    #[error_chain(custom)]
    Idna(idna::uts46::Errors),
    /// JSON Parse Error
    #[error_chain(foreign)]
    Json(serde_json::error::Error),
    /// String UTF8 Parse Error
    #[error_chain(foreign)]
    StrParse(std::string::FromUtf8Error),
    /// URL Parse Error
    #[error_chain(foreign)]
    UrlParse(url::ParseError),
}

impl From<IdnaError> for Error {
    fn from(es: IdnaError) -> Error {
        ErrorKind::Idna(es).into()
    }
}

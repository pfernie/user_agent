extern crate cookie as raw_cookie;
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

use std::{error, fmt};

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
use serde_json::error::Error as JsonError;

use idna::uts46::Errors as IdnaError;
use std::io::Error as IoError;
use std::string::FromUtf8Error as Utf8Error;
use url::ParseError as UrlError;
#[derive(Debug)]
pub enum Error {
    /// IO Error
    Io(IoError),
    /// IDNA Parse Error
    Idna,
    /// JSON Parse Error
    Json(JsonError),
    /// String UTF8 Parse Error
    StrParse(Utf8Error),
    /// URL Parse Error
    UrlParse(UrlError),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::Idna => "IDNA error",
            Error::Json(ref err) => JsonError::description(err),
            Error::StrParse(ref err) => Utf8Error::description(err),
            Error::UrlParse(ref err) => UrlError::description(err),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Idna => None,
            Error::Json(ref err) => Some(err),
            Error::StrParse(ref err) => Some(err),
            Error::UrlParse(ref err) => Some(err),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::Idna => write!(f, "IDNA error"),
            Error::Json(ref err) => write!(f, "JSON error: {}", err),
            Error::StrParse(ref err) => write!(f, "Could not parse bytes as UTF-8 String: {}", err),
            Error::UrlParse(ref err) => write!(f, "Could not parse as URL: {}", err),
        }
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Error {
        Error::Io(error)
    }
}

impl From<IdnaError> for Error {
    fn from(_: IdnaError) -> Error {
        Error::Idna
    }
}

impl From<JsonError> for Error {
    fn from(error: JsonError) -> Error {
        Error::Json(error)
    }
}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Error {
        Error::StrParse(error)
    }
}

impl From<UrlError> for Error {
    fn from(error: UrlError) -> Error {
        Error::UrlParse(error)
    }
}

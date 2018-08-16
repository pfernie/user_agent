extern crate cookie as raw_cookie;
#[macro_use]
extern crate error_chain;
#[cfg(test)]
extern crate env_logger;
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
pub mod reqwest_session;
pub use reqwest_session::ReqwestSession;
mod utils;

use idna::uts46::Errors as IdnaError;
error_chain!{
    foreign_links {
        Io(std::io::Error);
        Json(serde_json::error::Error);
        StrParse(std::string::FromUtf8Error);
        UrlParse(url::ParseError);
        Reqwest(reqwest::Error);
    }

    errors {
        Idna(t: idna::uts46::Errors) {}
    }
}

impl From<IdnaError> for Error {
    fn from(es: IdnaError) -> Error {
        ErrorKind::Idna(es).into()
    }
}

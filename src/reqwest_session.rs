use crate::session::{Session, SessionClient, SessionRequest, SessionResponse};
use cookie::Cookie as RawCookie;
use log::debug;
use reqwest;
use reqwest::header::{COOKIE, SET_COOKIE};
use url::Url;

impl SessionResponse for reqwest::blocking::Response {
    type Url = url::Url;
    fn parse_set_cookie(&self) -> Vec<RawCookie<'static>> {
        self.headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|set_cookie| {
                set_cookie
                    .to_str()
                    .map_err(|e| {
                        debug!(
                            "error parsing Set-Cookie to String {:?}: {:?}",
                            set_cookie, e
                        );
                        e
                    })
                    .ok()
                    .and_then(|sc| match RawCookie::parse(sc.to_owned()) {
                        Ok(raw_cookie) => Some(raw_cookie),
                        Err(e) => {
                            debug!(
                                "error parsing Set-Cookie to RawCookie {:?}: {:?}",
                                set_cookie, e
                            );
                            None
                        }
                    })
            })
            .collect::<Vec<_>>()
    }

    fn final_url(&self) -> Option<&url::Url> {
        Some(&self.url())
    }
}

impl SessionRequest for reqwest::blocking::RequestBuilder {
    fn add_cookies(self, cookies: Vec<&RawCookie<'static>>) -> Self {
        if cookies.is_empty() {
            debug!("no cookies to add to request");
            self
        } else {
            let cookies = cookies.iter().map(|rc| rc.encoded().to_string());
            let mut out = self;
            for cookie in cookies {
                out = out.header(COOKIE, cookie);
            }
            out
        }
    }
}

#[derive(Debug)]
pub enum ReqwestSessionError {
    ParseUrlError(url::ParseError),
    ReqwestError(reqwest::Error),
}

impl std::fmt::Display for ReqwestSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ReqwestSessionError::ParseUrlError(e) => write!(f, "URL parse error: {}", e),
            ReqwestSessionError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
        }
    }
}

impl std::error::Error for ReqwestSessionError {}

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

pub type ReqwestSession = Session<reqwest::blocking::Client>;

impl SessionClient for reqwest::blocking::Client {
    type Request = reqwest::blocking::RequestBuilder;
    type Response = reqwest::blocking::Response;
    type SendError = ReqwestSessionError;

    fn get_request(&self, url: &Url) -> Self::Request {
        self.get(url.clone())
    }
    fn put_request(&self, url: &Url) -> Self::Request {
        self.put(url.clone())
    }
    fn head_request(&self, url: &Url) -> Self::Request {
        self.head(url.clone())
    }
    fn delete_request(&self, url: &Url) -> Self::Request {
        self.delete(url.clone())
    }
    fn post_request(&self, url: &Url) -> Self::Request {
        self.post(url.clone())
    }

    fn send(&self, request: Self::Request) -> Result<Self::Response, Self::SendError> {
        request.send().map_err(ReqwestSessionError::from)
    }
}

#[cfg(test)]
mod tests {
    use env_logger;
    use reqwest;

    use super::ReqwestSession;

    macro_rules! dump {
        ($e: expr, $i: ident) => {{
            use serde_json;
            use time::now_utc;
            println!("");
            println!("==== {}: {} ====", $e, now_utc().rfc3339());
            for c in $i.store.iter_any() {
                println!(
                    "{} {}",
                    if c.is_expired() {
                        "XXXXX"
                    } else if c.is_persistent() {
                        "PPPPP"
                    } else {
                        "     "
                    },
                    serde_json::to_string(c).unwrap()
                );
                println!("----------------");
            }
            println!("================");
        }};
    }

    fn assert_cookies_count(session: &mut ReqwestSession, url: &str, add: bool) {
        let cookies_count_origin = session.store.iter_unexpired().count();
        session
            .get(url)
            .unwrap_or_else(|_| panic!("session get {} failed", url));
        let cookies_count = session.store.iter_unexpired().count();
        let cookies_count_added = if add {
            reqwest::blocking::Client::new()
                .get(url)
                .send()
                .unwrap_or_else(|_| panic!("cilent get {} failed", url))
                .headers()
                .get_all(reqwest::header::SET_COOKIE)
                .iter()
                .count()
        } else {
            0
        };
        let cookies_count_expected = cookies_count_origin + cookies_count_added;
        assert_eq!(cookies_count, cookies_count_expected);
    }

    #[test]
    fn test_gets() {
        env_logger::init();
        let mut s = ReqwestSession::new(reqwest::blocking::Client::new());
        dump!("init", s);
        assert_cookies_count(&mut s, "http://www.google.com", true);
        dump!("after google", s);
        assert_cookies_count(&mut s, "http://www.google.com", false);
        dump!("after google again", s);
        // yahoo doesn't set any cookies; how nice of them
        assert_cookies_count(&mut s, "http://www.yahoo.com", false);
        dump!("after yahoo", s);
        assert_cookies_count(&mut s, "http://www.msn.com", true);
        dump!("after msn", s);
    }
}

use crate::cookie_store::CookieStore;
use crate::session::{Session, SessionClient, SessionRequest, SessionResponse};
use cookie::Cookie as RawCookie;
use log::debug;
use reqwest;
use reqwest::header::{COOKIE, SET_COOKIE};
use url::Url;

impl SessionResponse for reqwest::Response {
    fn parse_set_cookie(&self) -> Option<Vec<RawCookie<'static>>> {
        self.headers().get(SET_COOKIE).map(|set_cookie| {
            set_cookie
                .to_str()
                .iter()
                .filter_map(|h_c| match RawCookie::parse(h_c.to_string()) {
                    Ok(raw_cookie) => Some(raw_cookie),
                    Err(e) => {
                        debug!("error parsing Set-Cookie {:?}: {:?}", h_c, e);
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
    }

    fn final_url(&self) -> Option<&Url> {
        Some(self.url())
    }
}

impl SessionRequest for reqwest::RequestBuilder {
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

pub type ReqwestSession = Session<reqwest::Client>;

impl SessionClient for reqwest::Client {
    type Request = reqwest::RequestBuilder;
    type Response = reqwest::Response;
    type SendError = super::ReqwestSessionError;

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
}

impl ::std::ops::Deref for ReqwestSession {
    type Target = CookieStore;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl ::std::ops::DerefMut for ReqwestSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
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
            for c in $i.iter_any() {
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

    #[test]
    fn test_gets() {
        use super::super::ReqwestSessionError;

        env_logger::init();
        let mut s = ReqwestSession::new(reqwest::Client::new());
        dump!("init", s);
        s.get_with("http://www.google.com", |req| {
            req.send().map_err(ReqwestSessionError::from)
        })
        .expect("www.google.com get_with failed");
        let c1 = s.iter_unexpired().count();
        assert!(c1 > 0);
        s.get_with("http://www.google.com", |req| {
            req.send().map_err(ReqwestSessionError::from)
        })
        .expect("www.google.com get_with failed");
        assert!(c1 == s.iter_unexpired().count()); // no new cookies on re-request
        dump!("after google", s);
        s.get_with("http://www.yahoo.com", |req| {
            req.send().map_err(ReqwestSessionError::from)
        })
        .expect("www.yahoo.com get_with failed");
        dump!("after yahoo", s);
        let c2 = s.iter_unexpired().count();
        assert!(c2 > 0);
        assert!(c2 == c1); // yahoo doesn't set any cookies; how nice of them
        s.get_with("http://www.msn.com", |req| {
            req.send().map_err(ReqwestSessionError::from)
        })
        .expect("www.msn.com get_with failed");
        dump!("after msn", s);
        let c3 = s.iter_unexpired().count();
        assert!(c3 > 0);
        assert!(c3 > c2);
    }
}

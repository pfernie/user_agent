use cookie_store::CookieStore;
use hyper;
use hyper::client::response::Response as HyperResponse;
use hyper::header::{Header, SetCookie};
use hyper::header::Cookie as CookieHeader;
use raw_cookie::Cookie as RawCookie;
use session::{CarriesCookies, HasSetCookie, Session, SessionCookieStore, WithSession};
use url::Url;
use utils::IntoUrl;

impl HasSetCookie for HyperResponse {
    fn parse_set_cookie(&self) -> Vec<RawCookie> {
        if let Some(set_cookie) = self.headers.get::<SetCookie>() {
            // hyper is using cookie 0.1, we are on 0.2, so to_string()/parse() to get to
            // the
            // correct version
            set_cookie.iter()
                .filter_map(|h_c| {
                    match RawCookie::parse(&h_c.to_string()[..]) {
                        Ok(raw_cookie) => Some(raw_cookie),
                        Err(e) => {
                            debug!("error parsing Set-Cookie {:?}: {:?}", h_c, e);
                            None
                        }
                    }
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    }
}

impl<'a> CarriesCookies for hyper::client::RequestBuilder<'a> {
    fn add_cookies(self, cookies: Vec<&RawCookie>) -> Self {
        if 0 == cookies.len() {
            debug!("no cookies to add to request");
            self
        } else {
            // again, hyper cookie version mismatches ours, so need to do some tricks
            let cookie_bytes = &cookies.iter()
                .map(|rc| rc.pair().to_string().into_bytes())
                .collect::<Vec<_>>()[..];
            match CookieHeader::parse_header(cookie_bytes) {
                Ok(cookie_header) => {
                    debug!("setting Cookie Header for request: {:?}", cookie_header);
                    self.header(cookie_header)
                }
                Err(e) => {
                    debug!("error parsing cookie set for request: {}", e);
                    self
                }
            }
        }
    }
}

pub type HyperSession = Session<hyper::client::Client>;
impl<'b> WithSession<'b> for HyperSession {
    type Request = hyper::client::RequestBuilder<'b>;
    type Response = HyperResponse;
    type SendError = hyper::error::Error;

    define_req_with!(get_with,
                     hyper::client::Client::new(),
                     |url, &client| client.get(url.clone()));
    define_req_with!(head_with,
                     hyper::client::Client::new(),
                     |url, &client| client.head(url.clone()));
    define_req_with!(delete_with,
                     hyper::client::Client::new(),
                     |url, &client| client.delete(url.clone()));
    define_req_with!(post_with,
                     hyper::client::Client::new(),
                     |url, &client| client.post(url.clone()));
    define_req_with!(put_with,
                     hyper::client::Client::new(),
                     |url, &client| client.put(url.clone()));
}

impl ::std::ops::Deref for HyperSession {
    type Target = CookieStore;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl ::std::ops::DerefMut for HyperSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

#[cfg(test)]
mod tests {
    use env_logger;
    use hyper::client::Client as HyperClient;
    use session::WithSession;
    use super::HyperSession;

    macro_rules! dump {
        ($e: expr, $i: ident) => ({
            use time::now_utc;
            use serde_json;
            println!("");
            println!("==== {}: {} ====", $e, now_utc().rfc3339());
            for c in $i.iter_any() {
                println!("{} {}", if c.is_expired() { "XXXXX" } else if c.is_persistent() { "PPPPP" }else { "     " }, serde_json::to_string(c).unwrap());
                println!("----------------");
            }
            println!("================");
        })
    }

    #[test]
    fn test_gets() {
        fn run_get<'c>(s: &mut HyperSession,
                       url: &str)
                       -> Result<::hyper::client::response::Response, ::hyper::error::Error> {
            s.get_with(url, |req| req.send())
        }
        env_logger::init().unwrap();
        let mut s = HyperSession::new(HyperClient::new());
        dump!("init", s);
        run_get(&mut s, "http://www.google.com/").unwrap();
        let c1 = s.iter_unexpired().count();
        assert!(c1 > 0);
        run_get(&mut s, "http://www.google.com/").unwrap();
        assert!(c1 == s.iter_unexpired().count()); // no new cookies on re-request
        dump!("after google", s);
        run_get(&mut s, "http://www.yahoo.com/").unwrap();
        dump!("after yahoo", s);
        let c2 = s.iter_unexpired().count();
        assert!(c2 > 0);
        assert!(c2 == c1); // yahoo doesn't set any cookies; how nice of them
        run_get(&mut s, "http://www.msn.com/").unwrap();
        dump!("after msn", s);
        let c3 = s.iter_unexpired().count();
        assert!(c3 > 0);
        assert!(c3 > c2);
    }
}

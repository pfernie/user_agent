use cookie_store::CookieStore;
use reqwest;
use reqwest::header::{Header, SetCookie};
use reqwest::header::Cookie as CookieHeader;
use raw_cookie::Cookie as RawCookie;
use session::{CarriesCookies, HasSetCookie, Session, SessionCookieStore, WithSession};
use url::Url;
use utils::IntoUrl;

impl HasSetCookie for reqwest::Response {
    fn parse_set_cookie(&self) -> Vec<RawCookie<'static>> {
        if let Some(set_cookie) = self.headers().get::<SetCookie>() {
            // reqwest is using cookie 0.2, we are on 0.4, so to_string()/parse() to get to
            // the
            // correct version
            set_cookie
                .iter()
                .filter_map(|h_c| match RawCookie::parse(h_c.to_string()) {
                                Ok(raw_cookie) => Some(raw_cookie),
                                Err(e) => {
                    debug!("error parsing Set-Cookie {:?}: {:?}", h_c, e);
                    None
                }
                            })
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    }
}

impl CarriesCookies for reqwest::RequestBuilder {
    fn add_cookies(self, cookies: Vec<&RawCookie<'static>>) -> Self {
        if 0 == cookies.len() {
            debug!("no cookies to add to request");
            self
        } else {
            // again, reqwest cookie version mismatches ours, so need to do some tricks
            let cookie_bytes = &cookies
                                    .iter()
                                    .map(|rc| rc.encoded().to_string().into_bytes())
                                    .collect::<Vec<_>>()
                                    [..];
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

pub type ReqwestSession = Session<reqwest::Client>;

impl<'b> WithSession<'b> for ReqwestSession {
    type Request = reqwest::RequestBuilder;
    type Response = reqwest::Response;
    type SendError = reqwest::Error;

    define_req_with!(get_with, |url, &client| client.get(url.clone()));
    define_req_with!(head_with, |url, &client| client.head(url.clone()));

    fn delete_with<U, P>(&'b mut self, _: U, _: P) -> Result<Self::Response, Self::SendError>
        where U: IntoUrl,
              P: FnOnce(Self::Request) -> Result<Self::Response, Self::SendError>
    {
        unimplemented!()
    }
    define_req_with!(post_with, |url, &client| client.post(url.clone()));
    fn put_with<U, P>(&'b mut self, _: U, _: P) -> Result<Self::Response, Self::SendError>
        where U: IntoUrl,
              P: FnOnce(Self::Request) -> Result<Self::Response, Self::SendError>
    {
        unimplemented!()
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

    use session::WithSession;
    use super::ReqwestSession;

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
        env_logger::init().unwrap();
        let mut s = ReqwestSession::new(reqwest::Client::new().expect("unable to create Client"));
        dump!("init", s);
        s.get_with("http://www.google.com", |req| req.send())
            .expect("www.google.com get_with failed");
        let c1 = s.iter_unexpired().count();
        assert!(c1 > 0);
        s.get_with("http://www.google.com", |req| req.send())
            .expect("www.google.com get_with failed");
        assert!(c1 == s.iter_unexpired().count()); // no new cookies on re-request
        dump!("after google", s);
        s.get_with("http://www.yahoo.com", |req| req.send())
            .expect("www.yahoo.com get_with failed");
        dump!("after yahoo", s);
        let c2 = s.iter_unexpired().count();
        assert!(c2 > 0);
        assert!(c2 == c1); // yahoo doesn't set any cookies; how nice of them
        s.get_with("http://www.msn.com", |req| req.send())
            .expect("www.msn.com get_with failed");
        dump!("after msn", s);
        let c3 = s.iter_unexpired().count();
        assert!(c3 > 0);
        assert!(c3 > c2);
    }
}

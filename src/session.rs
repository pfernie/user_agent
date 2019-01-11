use crate::utils::IntoUrl;
use cookie::Cookie as RawCookie;
use cookie_store::{Cookie, CookieStore};
use std::io::{BufRead, Write};
use url::{ParseError as ParseUrlError, Url};

/// Trait representing requests which can carry a Cookie header, appropriate
/// for use with a `Session`
pub trait SessionRequest {
    /// Add the given set of cookies to the request
    fn add_cookies(self, _: Vec<&RawCookie<'static>>) -> Self;
}

/// Trait representing responses which may have a Set-Cookie header, appropriate
/// for use with a `Session`
pub trait SessionResponse {
    /// Parse the Set-Cookie header and return the set of cookies if present
    fn parse_set_cookie(&self) -> Option<Vec<RawCookie<'static>>>;
    /// Return the final Url for the response. In cases such as redirects,
    /// such Url may differ from the Request Url. May return `None` if unavailable.
    fn final_url(&self) -> Option<&Url>;
}

macro_rules! define_with_fn {
    ($with_fn: ident, $request_fn: ident) => {
    pub fn $with_fn<U, P>(
        &mut self,
        url: U,
        prepare_and_send: P,
    ) -> ::std::result::Result<<C as SessionClient>::Response, <C as SessionClient>::SendError>
    where
        P: FnOnce(<C as SessionClient>::Request) -> ::std::result::Result<<C as SessionClient>::Response, <C as SessionClient>::SendError>,
        U: IntoUrl
    {
        let url = url.into_url()?;
        let request = self.client.$request_fn(&url);
        self.run_request(request, &url, prepare_and_send)
    }
    }
}

/// Trait representing the typical HTTP request methods, to be implemented
/// for clients appropriate for use in a `Session`
pub trait SessionClient {
    type Request: SessionRequest;
    type Response: SessionResponse;
    type SendError: From<ParseUrlError>;

    /// Create a `Self::Request` for a GET request
    fn get_request(&self, url: &Url) -> Self::Request;
    /// Create a `Self::Request` for a PUT request
    fn put_request(&self, url: &Url) -> Self::Request;
    /// Create a `Self::Request` for a HEAD request
    fn head_request(&self, url: &Url) -> Self::Request;
    /// Create a `Self::Request` for a DELETE request
    fn delete_request(&self, url: &Url) -> Self::Request;
    /// Create a `Self::Request` for a POST request
    fn post_request(&self, url: &Url) -> Self::Request;
}

pub struct Session<C: SessionClient> {
    pub client: C,
    pub store: CookieStore,
}

impl<C: SessionClient> Session<C> {
    pub fn new(client: C) -> Self {
        Session {
            client,
            store: CookieStore::default(),
        }
    }

    pub fn load<R, E, F>(
        client: C,
        reader: R,
        cookie_from_str: F,
    ) -> Result<Session<C>, failure::Error>
    where
        R: BufRead,
        F: Fn(&str) -> ::std::result::Result<Cookie<'static>, E>,
        failure::Error: From<E>,
    {
        let store = CookieStore::load(reader, cookie_from_str)?;
        Ok(Session { client, store })
    }

    pub fn load_json<R: BufRead>(client: C, reader: R) -> Result<Session<C>, failure::Error> {
        let store = CookieStore::load_json(reader)?;
        Ok(Session { client, store })
    }

    pub fn save<W, E, F>(&self, writer: &mut W, cookie_to_string: F) -> Result<(), failure::Error>
    where
        W: Write,
        F: Fn(&Cookie<'_>) -> ::std::result::Result<String, E>,
        failure::Error: From<E>,
    {
        self.store.save(writer, cookie_to_string)
    }

    pub fn save_json<W: Write>(&self, writer: &mut W) -> Result<(), failure::Error> {
        self.store.save_json(writer)
    }

    define_with_fn!(get_with, get_request);
    define_with_fn!(put_with, put_request);
    define_with_fn!(head_with, head_request);
    define_with_fn!(delete_with, delete_request);
    define_with_fn!(post_with, post_request);

    fn run_request<P>(
        &mut self,
        request: <C as SessionClient>::Request,
        url: &Url,
        prepare_and_send: P,
    ) -> ::std::result::Result<<C as SessionClient>::Response, <C as SessionClient>::SendError>
    where
        P: FnOnce(
            <C as SessionClient>::Request,
        ) -> ::std::result::Result<
            <C as SessionClient>::Response,
            <C as SessionClient>::SendError,
        >,
    {
        let response = {
            let cookies = self.store.get_request_cookies(url).collect();
            let request = request.add_cookies(cookies);
            prepare_and_send(request)?
        };
        if let Some(cookies) = response.parse_set_cookie() {
            let final_url: &Url = response.final_url().unwrap_or(url);
            self.store
                .store_response_cookies(cookies.into_iter(), final_url);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::{Session, SessionClient, SessionRequest, SessionResponse};
    use cookie::Cookie as RawCookie;
    use cookie_store::CookieStore;
    use std::io::{self, Read};
    use url::ParseError as ParseUrlError;
    use url::Url;

    // stolen example from hyper...
    /// An enum of possible body types for a Request.
    pub enum Body<'b> {
        /// A Reader does not necessarily know it's size, so it is chunked.
        ChunkedBody(&'b mut (dyn Read + 'b)),
        // /// For Readers that can know their size, like a `File`.
        // SizedBody(&'b mut (Read + 'b), u64),
        /// A String has a size, and uses Content-Length.
        BufBody(&'b [u8], usize),
    }

    impl<'b> Read for Body<'b> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match *self {
                Body::ChunkedBody(ref mut r) => r.read(buf),
                // Body::SizedBody(ref mut r, _) => r.read(buf),
                Body::BufBody(ref mut r, _) => Read::read(r, buf),
            }
        }
    }

    impl<'b> Into<Body<'b>> for &'b [u8] {
        #[inline]
        fn into(self) -> Body<'b> {
            Body::BufBody(self, self.len())
        }
    }

    impl<'b> Into<Body<'b>> for &'b str {
        #[inline]
        fn into(self) -> Body<'b> {
            self.as_bytes().into()
        }
    }

    impl<'b> Into<Body<'b>> for &'b String {
        #[inline]
        fn into(self) -> Body<'b> {
            self.as_bytes().into()
        }
    }

    impl<'b, R: Read> From<&'b mut R> for Body<'b> {
        #[inline]
        fn from(r: &'b mut R) -> Body<'b> {
            Body::ChunkedBody(r)
        }
    }

    impl<'b> SessionRequest for TestClientRequest<'b> {
        fn add_cookies(mut self, cookies: Vec<&RawCookie<'static>>) -> Self {
            for cookie in cookies.into_iter() {
                self.cookies.push(cookie.clone());
            }
            self
        }
    }

    struct TestClientRequest<'b> {
        cookies: Vec<RawCookie<'static>>,
        outgoing: Vec<RawCookie<'static>>,
        body: Option<Body<'b>>,
    }

    impl<'b> TestClientRequest<'b> {
        fn set_body<B: Into<Body<'b>>>(&mut self, body: B) {
            self.body = Some(body.into());
        }

        fn set_outgoing(&mut self, cookies: Vec<RawCookie<'static>>) {
            self.outgoing = cookies;
        }

        fn send(self) -> Result<TestClientResponse, TestError> {
            Ok(TestClientResponse(
                match self.body {
                    Some(mut body) => {
                        let mut b = String::new();
                        body.read_to_string(&mut b).unwrap();
                        format!("body was: '{}'", b)
                    }
                    None => "no body sent".to_string(),
                },
                self.outgoing,
            ))
        }
    }

    struct TestClientResponse(String, Vec<RawCookie<'static>>);
    impl SessionResponse for TestClientResponse {
        fn parse_set_cookie(&self) -> Option<Vec<RawCookie<'static>>> {
            Some(self.1.clone())
        }

        fn final_url(&self) -> Option<&Url> {
            None
        }
    }

    impl TestClientResponse {
        pub fn body(self) -> String {
            self.0
        }
    }

    struct TestClient;
    impl TestClient {
        fn request(&self, _: &Url) -> TestClientRequest<'_> {
            TestClientRequest {
                cookies: vec![],
                outgoing: vec![],
                body: None,
            }
        }
    }

    impl<'b> SessionClient for &'b TestClient {
        type Request = TestClientRequest<'b>;
        type Response = TestClientResponse;
        type SendError = TestError;

        fn get_request(&self, url: &Url) -> Self::Request {
            self.request(url)
        }
        fn put_request(&self, url: &Url) -> Self::Request {
            self.request(url)
        }
        fn head_request(&self, url: &Url) -> Self::Request {
            self.request(url)
        }
        fn delete_request(&self, url: &Url) -> Self::Request {
            self.request(url)
        }
        fn post_request(&self, url: &Url) -> Self::Request {
            self.request(url)
        }
    }

    type TestSession<'c> = Session<&'c TestClient>;

    #[derive(Debug, Clone, PartialEq)]
    struct TestError;
    use std::error;
    impl error::Error for TestError {
        fn description(&self) -> &str {
            "TestError"
        }
        fn cause(&self) -> Option<&dyn error::Error> {
            None
        }
    }
    use std::fmt;
    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "test error")
        }
    }
    impl From<ParseUrlError> for TestError {
        fn from(_: ParseUrlError) -> TestError {
            TestError
        }
    }

    #[allow(unused_macros)]
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

    macro_rules! is_in_vec {
        ($i: ident, $e: expr) => {
            assert!($i.iter().any(|c| c.name() == $e));
        };
    }

    macro_rules! value_in_vec {
        ($i: ident, $e: expr, $v: expr) => {
            assert!($i.iter().find(|c| c.name() == $e).unwrap().value() == $v);
        };
    }

    macro_rules! not_in_vec {
        ($i: ident, $e: expr) => {
            assert!(!$i.iter().any(|c| c.name() == $e));
        };
    }

    macro_rules! has_sess {
        ($store: ident, $d: expr, $p: expr, $n: expr) => {
            assert!(!$store.get($d, $p, $n).unwrap().is_persistent());
        };
    }

    macro_rules! has_pers {
        ($store: ident, $d: expr, $p: expr, $n: expr) => {
            assert!($store.get($d, $p, $n).unwrap().is_persistent());
        };
    }

    macro_rules! has_expired {
        ($store: ident, $d: expr, $p: expr, $n: expr) => {
            assert!($store.contains_any($d, $p, $n) && !$store.contains($d, $p, $n));
        };
    }

    macro_rules! has_value {
        ($store: ident, $d: expr, $p: expr, $n: expr, $v: expr) => {
            assert_eq!($store.get($d, $p, $n).unwrap().value(), $v);
        };
    }

    macro_rules! not_has {
        ($store: ident, $n: expr) => {
            assert_eq!($store.iter_any().filter(|c| c.name() == $n).count(), 0);
        };
    }

    macro_rules! load_session {
        ($s: ident, $c: expr, $sd: ident) => {
            let mut $s = Session::load_json($c, &$sd[..]).unwrap();
        };
    }

    macro_rules! save_session {
        ($s: ident) => {{
            let mut output = vec![];
            $s.save_json(&mut output).unwrap();
            output
        }};
    }

    impl<'s> ::std::ops::Deref for TestSession<'s> {
        type Target = CookieStore;
        fn deref(&self) -> &Self::Target {
            &self.store
        }
    }

    impl<'s> ::std::ops::DerefMut for TestSession<'s> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.store
        }
    }

    #[test]
    fn client() {
        let session1 = {
            // init & try http://www.example.com
            let mut s = TestSession::new(&TestClient);
            let url = Url::parse("http://www.example.com").unwrap();

            s.parse("0=_", &url).unwrap();
            s.parse("1=a; Max-Age=120", &url).unwrap();
            s.parse("2=b; Max-Age=120", &url).unwrap();
            s.parse("secure=zz; Max-Age=120; Secure", &url).unwrap();
            s.parse(
                "foo_domain=zzz",
                &Url::parse("http://foo.example.com").unwrap(),
            )
            .unwrap(); // should not be included in our www.example.com request
            s.parse(
                "foo_domain_pers=zzz; Max-Age=120",
                &Url::parse("http://foo.example.com").unwrap(),
            )
            .unwrap(); // should not be included in our www.example.com request
            has_sess!(s, "www.example.com", "/", "0");
            has_pers!(s, "www.example.com", "/", "1");
            has_pers!(s, "www.example.com", "/", "2");
            has_pers!(s, "www.example.com", "/", "secure"); // it should be parsed, but not included in our non-https request
            has_sess!(s, "foo.example.com", "/", "foo_domain"); // it should be parsed, but not included in our non-foo.example.com request
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers"); // it should be parsed, but not included in our non-foo.example.com request

            let body = "this is the body".to_string();
            let resp = s
                .get_with("http://www.example.com", |mut r| {
                    let incoming = r.cookies.clone();
                    is_in_vec!(incoming, "0");
                    is_in_vec!(incoming, "1");
                    is_in_vec!(incoming, "2");
                    not_in_vec!(incoming, "3"); // hasn't been set yet...
                    not_in_vec!(incoming, "secure"); // not a secure request
                    not_in_vec!(incoming, "foo_domain"); // wrong domain
                    not_in_vec!(incoming, "foo_domain_pers"); // wrong domain
                    r.set_body(&body);
                    r.set_outgoing(vec![
                        RawCookie::parse("0=hi").unwrap(), // update the non-persistent 0 cookie
                        RawCookie::parse("1=sess1; Max-Age=120").unwrap(), // update the 1 persistent cookie
                        RawCookie::parse("2=c; Max-Age=0").unwrap(),       // expire the 2 cookie
                        RawCookie::parse("3=c").unwrap(),                  // new session cookie
                        RawCookie::parse("4=d; Max-Age=0").unwrap(), // add an expired cookie, should never show up
                        RawCookie::parse("5=e; Domain=invalid.com").unwrap(), // invalid domain, should never show up
                        RawCookie::parse("6=f; Domain=example.com").unwrap(), // should be able to set for a higher domain
                        RawCookie::parse("7=g; Max-Age=300").unwrap(), // new persistent (5min) cookie
                    ]);
                    r.send()
                })
                .unwrap();
            assert_eq!("body was: 'this is the body'", resp.body());

            has_sess!(s, "www.example.com", "/", "0"); // was existing
            has_value!(s, "www.example.com", "/", "0", "hi"); // check the value was set properly
            has_pers!(s, "www.example.com", "/", "1"); // was existing
            has_value!(s, "www.example.com", "/", "1", "sess1"); // check the value was set properly
            has_expired!(s, "www.example.com", "/", "2"); // expired by response
            has_sess!(s, "www.example.com", "/", "3"); // session-only added by response
            has_value!(s, "www.example.com", "/", "3", "c"); // check the value was set properly
            not_has!(s, "4"); // expired on insert, so not added
            not_has!(s, "5"); // invalid domain
            has_sess!(s, "example.com", "/", "6"); // non-persistent cookie set for higher domain
            has_pers!(s, "www.example.com", "/", "7"); // persistent added by response
            has_pers!(s, "www.example.com", "/", "secure"); // verify cookies not included in request still present
            has_sess!(s, "foo.example.com", "/", "foo_domain"); // verify cookies not included in request still present
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers"); // "

            save_session!(s)
        };

        let session2 = {
            // try https://www.example.com - secure
            load_session!(s, &TestClient, session1);
            not_has!(s, "0"); // non-persistent cookie
            has_pers!(s, "www.example.com", "/", "1"); // was an initial persistent cookie
            has_value!(s, "www.example.com", "/", "1", "sess1");
            not_has!(s, "2"); // was expired during last session
            not_has!(s, "3"); // was not a persistent cookie
            not_has!(s, "4"); // expired cookie never set
            not_has!(s, "5"); // invalid domain
            not_has!(s, "6"); // was NOT a persistent cookie
            has_pers!(s, "www.example.com", "/", "7"); // was a persistent cookie
            has_pers!(s, "www.example.com", "/", "secure"); // it should be parsed, but not included in our non-https request
            not_has!(s, "foo_domain"); // it was parsed, but not persistent
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers"); // was parsed, not included, and persistent

            let resp = s
                .get_with("https://www.example.com", |mut r| {
                    let incoming = r.cookies.clone();
                    not_in_vec!(incoming, "0");
                    is_in_vec!(incoming, "1");
                    not_in_vec!(incoming, "2");
                    not_in_vec!(incoming, "3"); // was set last session, but not persistent
                    not_in_vec!(incoming, "4"); // was expired when set
                    not_in_vec!(incoming, "5"); // invalid domain
                    not_in_vec!(incoming, "6"); // not persistent
                    is_in_vec!(incoming, "7"); // persistent
                    is_in_vec!(incoming, "secure"); // a secure request, so included
                    not_in_vec!(incoming, "foo_domain"); // wrong domain, non-persistent anyway
                    not_in_vec!(incoming, "foo_domain_pers"); // wrong domain
                    r.set_body("this is the second body");
                    r.set_outgoing(vec![
                        RawCookie::parse("1=sess2; Max-Age=120").unwrap(), // update the 1 persistent cookie
                        RawCookie::parse("secure=ZZ; Max-Age=120").unwrap(), // update the secure cookie
                        RawCookie::parse("2=B; Max-Age=120; Path=/foo").unwrap(), // re-add the 2-cookie, but for a sub-path
                        RawCookie::parse("8=h; Domain=example.com").unwrap(), // should be able to set persistent for a higher domain
                    ]);
                    r.send()
                })
                .unwrap();
            assert_eq!("body was: 'this is the second body'", resp.body());

            not_has!(s, "0");
            has_pers!(s, "www.example.com", "/", "1");
            has_value!(s, "www.example.com", "/", "1", "sess2");
            has_value!(s, "www.example.com", "/foo", "2", "B"); // was re-set by response
            not_has!(s, "3");
            not_has!(s, "4");
            not_has!(s, "5");
            not_has!(s, "6");
            has_pers!(s, "www.example.com", "/", "7");
            has_sess!(s, "example.com", "/", "8"); // session cookie added by response
            has_value!(s, "www.example.com", "/", "secure", "ZZ"); // value was updated in response
            not_has!(s, "foo_domain");
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers");

            save_session!(s)
        };

        let session3 = {
            // try https://foo.example.com - secure & foo. subdomain
            load_session!(s, &TestClient, session2);
            not_has!(s, "0");
            has_pers!(s, "www.example.com", "/", "1");
            has_value!(s, "www.example.com", "/", "1", "sess2");
            has_value!(s, "www.example.com", "/foo", "2", "B");
            not_has!(s, "3");
            not_has!(s, "4");
            not_has!(s, "5");
            not_has!(s, "6");
            has_pers!(s, "www.example.com", "/", "7");
            not_has!(s, "8");
            has_value!(s, "www.example.com", "/", "secure", "ZZ");
            not_has!(s, "foo_domain");
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers");

            let resp = s
                .get_with("http://foo.example.com", |mut r| {
                    let incoming = r.cookies.clone();
                    not_in_vec!(incoming, "0");
                    not_in_vec!(incoming, "1"); // wrong domain
                    not_in_vec!(incoming, "2"); // newly added, but wrong domain & path
                    not_in_vec!(incoming, "3"); // was set last session, wrong domain, not persistent
                    not_in_vec!(incoming, "4"); // was expired when set
                    not_in_vec!(incoming, "5"); // invalid domain
                    not_in_vec!(incoming, "6"); // higher level domain, but not persistent
                    not_in_vec!(incoming, "7"); // persistent, but wrong domain
                    not_in_vec!(incoming, "8"); // not-persistent, higher level domain
                    not_in_vec!(incoming, "secure"); // secure request, but wrong domain
                    not_in_vec!(incoming, "foo_domain"); // correct domain, but non-persistent
                    is_in_vec!(incoming, "foo_domain_pers"); // correct domain, persistent
                    r.set_outgoing(vec![
                        RawCookie::parse("1=sess3; Max-Age=120").unwrap(), // set a new 1 cookie for foo.example.com
                        RawCookie::parse("secure=YY; Max-Age=120; Secure").unwrap(), // this secure cookie is for foo.example.com
                        RawCookie::parse("9=v; Domain=example.com; Path=/foo; Max-Age=120; Secure")
                            .unwrap(), // this secure cookie is for example.com/foo
                    ]);
                    r.send()
                })
                .unwrap();
            assert_eq!("no body sent", resp.body());

            not_has!(s, "0");
            has_pers!(s, "www.example.com", "/", "1");
            has_value!(s, "www.example.com", "/", "1", "sess2");
            has_pers!(s, "foo.example.com", "/", "1");
            has_value!(s, "foo.example.com", "/", "1", "sess3");
            has_value!(s, "www.example.com", "/foo", "2", "B");
            not_has!(s, "3");
            not_has!(s, "4");
            not_has!(s, "5");
            not_has!(s, "6");
            has_pers!(s, "www.example.com", "/", "7");
            not_has!(s, "8");
            has_value!(s, "www.example.com", "/", "secure", "ZZ");
            has_value!(s, "foo.example.com", "/", "secure", "YY");
            not_has!(s, "foo_domain");
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers");

            save_session!(s)
        };

        let session4 = {
            // try https://www.example.com/foo - secure & path
            load_session!(s, &TestClient, session3);
            not_has!(s, "0");
            has_pers!(s, "www.example.com", "/", "1");
            has_value!(s, "www.example.com", "/", "1", "sess2");
            has_pers!(s, "foo.example.com", "/", "1");
            has_value!(s, "foo.example.com", "/", "1", "sess3");
            has_value!(s, "www.example.com", "/foo", "2", "B");
            not_has!(s, "3");
            not_has!(s, "4");
            not_has!(s, "5");
            not_has!(s, "6");
            has_pers!(s, "www.example.com", "/", "7");
            not_has!(s, "8");
            has_pers!(s, "example.com", "/foo", "9");
            has_value!(s, "www.example.com", "/", "secure", "ZZ");
            has_value!(s, "foo.example.com", "/", "secure", "YY");
            not_has!(s, "foo_domain");
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers");

            s.get_with("https://www.example.com/foo", |r| {
                let incoming = r.cookies.clone();
                not_in_vec!(incoming, "0");
                is_in_vec!(incoming, "1");
                is_in_vec!(incoming, "2");
                not_in_vec!(incoming, "3");
                not_in_vec!(incoming, "4");
                not_in_vec!(incoming, "5");
                not_in_vec!(incoming, "6");
                is_in_vec!(incoming, "7");
                not_in_vec!(incoming, "8");
                is_in_vec!(incoming, "9");
                value_in_vec!(incoming, "secure", "ZZ"); // got the correct secure cookie
                not_in_vec!(incoming, "foo_domain");
                not_in_vec!(incoming, "foo_domain_pers");
                // no outgoing cookies
                r.send()
            })
            .unwrap();
            save_session!(s)
        };

        let session5 = {
            // try https://www.example.com/foo/bar - secure & deeper path
            load_session!(s, &TestClient, session4);
            not_has!(s, "0");
            has_pers!(s, "www.example.com", "/", "1");
            has_value!(s, "www.example.com", "/", "1", "sess2");
            has_pers!(s, "foo.example.com", "/", "1");
            has_value!(s, "foo.example.com", "/", "1", "sess3");
            has_value!(s, "www.example.com", "/foo", "2", "B");
            not_has!(s, "3");
            not_has!(s, "4");
            not_has!(s, "5");
            not_has!(s, "6");
            has_pers!(s, "www.example.com", "/", "7");
            not_has!(s, "8");
            has_pers!(s, "example.com", "/foo", "9");
            has_pers!(s, "example.com", "/foo", "9");
            has_value!(s, "www.example.com", "/", "secure", "ZZ");
            has_value!(s, "foo.example.com", "/", "secure", "YY");
            not_has!(s, "foo_domain");
            has_pers!(s, "foo.example.com", "/", "foo_domain_pers");

            s.get_with("https://www.example.com/foo/bar", |r| {
                let incoming = r.cookies.clone();
                not_in_vec!(incoming, "0");
                is_in_vec!(incoming, "1");
                is_in_vec!(incoming, "2");
                not_in_vec!(incoming, "3");
                not_in_vec!(incoming, "4");
                not_in_vec!(incoming, "5");
                not_in_vec!(incoming, "6");
                is_in_vec!(incoming, "7");
                not_in_vec!(incoming, "8");
                is_in_vec!(incoming, "9"); // validating that /foo/bar sees the /foo cookie
                value_in_vec!(incoming, "secure", "ZZ"); // got the correct secure cookie
                not_in_vec!(incoming, "foo_domain");
                not_in_vec!(incoming, "foo_domain_pers");
                // no outgoing cookies
                r.send()
            })
            .unwrap();
            save_session!(s)
        };

        load_session!(s, &TestClient, session5);
        s.get_with("https://www.example.com/", |r| {
            let incoming = r.cookies.clone();
            not_in_vec!(incoming, "9"); // validating that we don't see /foo cookie
                                        // no outgoing cookies
            r.send()
        })
        .unwrap();
        s.get_with("https://www.example.com/bar", |r| {
            let incoming = r.cookies.clone();
            not_in_vec!(incoming, "9"); // validating that we don't see /foo cookie
                                        // no outgoing cookies
            r.send()
        })
        .unwrap();
    }
}

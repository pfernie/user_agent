use std;

use idna;
use raw_cookie::Cookie as RawCookie;
use try_from::TryFrom;
use url::{Host, Url};

use ::Error;
use utils::is_host_name;

pub fn is_match(domain: &str, request_url: &Url) -> bool {
    CookieDomain::try_from(domain).map(|domain| domain.matches(request_url)).unwrap_or(false)
}

/// The domain of a `Cookie`
#[derive(PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CookieDomain {
    /// No Domain attribute in Set-Cookie header
    HostOnly(String),
    /// Domain attribute from Set-Cookie header
    Suffix(String),
    /// Domain attribute was not present in the Set-Cookie header
    NotPresent,
    /// Domain attribute-value was empty; technically undefined behavior, but suggested that this
    /// be treated as invalid
    Empty,
}

// 5.1.3.  Domain Matching
// A string domain-matches a given domain string if at least one of the
// following conditions hold:
//
// o  The domain string and the string are identical.  (Note that both
//    the domain string and the string will have been canonicalized to
//    lower case at this point.)
//
// o  All of the following conditions hold:
//
//    *  The domain string is a suffix of the string.
//
//    *  The last character of the string that is not included in the
//       domain string is a %x2E (".") character.
//
//    *  The string is a host name (i.e., not an IP address).
/// The concept of a domain match per [IETF RFC6265 Section
/// 5.1.3](http://tools.ietf.org/html/rfc6265#section-5.1.3)
impl CookieDomain {
    /// Tests if the given `url::Url` meets the domain-match criteria
    pub fn matches(&self, request_url: &Url) -> bool {
        if let Some(url_host) = request_url.host_str() {
            match *self {
                CookieDomain::HostOnly(ref host) => host == url_host,
                CookieDomain::Suffix(ref suffix) => {
                    suffix == url_host ||
                    (is_host_name(url_host) && url_host.ends_with(suffix) &&
                     url_host[(url_host.len() - suffix.len() - 1)..].starts_with("."))
                }
                CookieDomain::NotPresent | CookieDomain::Empty => false, // nothing can match the Empty case
            }
        } else {
            false // not a matchable scheme
        }
    }

    pub fn into_cow(&self) -> std::borrow::Cow<str> {
        match *self {
            CookieDomain::HostOnly(ref h) => std::borrow::Cow::Borrowed(h),
            CookieDomain::Suffix(ref s) => std::borrow::Cow::Borrowed(s),
            CookieDomain::Empty | CookieDomain::NotPresent => {
                panic!("cannot create Cow<'a, str> from CookieDomain::{Empty,NotPresnt}")
            }
        }
    }
}

/// Construct a `CookieDomain::Suffix` from a string, stripping a single leading '.' if present.
/// If the source string is empty, returns the `CookieDomain::Empty` variant.
impl<'a> TryFrom<&'a str> for CookieDomain {
    type Err = Error;
    fn try_from(value: &str) -> Result<CookieDomain, Self::Err> {
        idna::domain_to_ascii(value.trim())
            .map_err(|_| Error::Idna)
            .map(|domain| if domain.is_empty() {
                CookieDomain::Empty
            } else {
                CookieDomain::Suffix(domain)
            })
    }
}

/// Construct a `CookieDomain::Suffix` from a `cookie::Cookie`, which handles stripping a leading
/// '.' for us. If the cookie.domain is None or an empty string, the `CookieDomain::Empty` variant
/// is returned.
/// __NOTE__: `cookie::Cookie` domain values already have the leading '.' stripped. To avoid
/// performing this step twice, the `From<&cookie::Cookie>` impl should be used,
/// instead of passing `cookie.domain` to the `From<&str>` impl.
impl<'a> TryFrom<&'a RawCookie> for CookieDomain {
    type Err = Error;
    fn try_from(cookie: &'a RawCookie) -> Result<CookieDomain, Self::Err> {
        if let Some(ref domain) = cookie.domain {
            idna::domain_to_ascii(domain.trim())
                .map_err(|_| Error::Idna)
                .map(|domain| if domain.is_empty() {
                    CookieDomain::Empty
                } else {
                    CookieDomain::Suffix(domain)
                })
        } else {
            Ok(CookieDomain::NotPresent)
        }
    }
}

/// Construct a `CookieDomain::HostOnly` from a `url::Host`
impl<'a> TryFrom<Host<&'a str>> for CookieDomain {
    type Err = Error;
    fn try_from(h: Host<&'a str>) -> Result<CookieDomain, Self::Err> {
        Ok(match h {
            Host::Domain(d) => CookieDomain::HostOnly(d.into()),
            Host::Ipv4(addr) => CookieDomain::HostOnly(format!("{}", addr)),
            Host::Ipv6(addr) => CookieDomain::HostOnly(format!("[{}]", addr)),
        })
    }
}

impl<'a> From<&'a CookieDomain> for String {
    fn from(c: &'a CookieDomain) -> String {
        match *c {
            CookieDomain::HostOnly(ref h) => h.to_owned(),
            CookieDomain::Suffix(ref s) => s.to_owned(),
            CookieDomain::Empty | CookieDomain::NotPresent => "".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use raw_cookie::Cookie as RawCookie;
    use try_from::TryFrom;
    use url::Url;

    use super::CookieDomain;
    use utils::test::*;

    #[inline]
    fn matches(expected: bool, cookie_domain: &CookieDomain, url: &str) {
        let url = Url::parse(url).unwrap();
        assert!(expected == cookie_domain.matches(&url),
                "cookie_domain: {:?} url: {:?}, url.host_str(): {:?}",
                cookie_domain,
                url,
                url.host_str());
    }

    #[inline]
    fn variants(expected: bool, cookie_domain: &CookieDomain, url: &str) {
        matches(expected, cookie_domain, url);
        matches(expected, cookie_domain, &format!("{}/", url));
        matches(expected, cookie_domain, &format!("{}:8080", url));
        matches(expected, cookie_domain, &format!("{}/foo/bar", url));
        matches(expected, cookie_domain, &format!("{}:8080/foo/bar", url));
    }

    #[test]
    fn matches_hostonly() {
        {
            let url = url("http://example.com");
            // HostOnly must be an identical string match, and may be an IP address
            // or a hostname
            let host_name = CookieDomain::try_from(url.host().unwrap())
                .expect("unable to parse domain");
            matches(false, &host_name, "data:nonrelative");
            variants(true, &host_name, "http://example.com");
            variants(false, &host_name, "http://example.org");
            // per RFC6265:
            //    WARNING: Some existing user agents treat an absent Domain
            //      attribute as if the Domain attribute were present and contained
            //      the current host name.  For example, if example.com returns a Set-
            //      Cookie header without a Domain attribute, these user agents will
            //      erroneously send the cookie to www.example.com as well.
            variants(false, &host_name, "http://foo.example.com");
            variants(false, &host_name, "http://127.0.0.1");
            variants(false, &host_name, "http://[::1]");
        }

        {
            let url = url("http://127.0.0.1");
            let ip4 = CookieDomain::try_from(url.host().unwrap()).expect("unable to parse Ipv4");
            matches(false, &ip4, "data:nonrelative");
            variants(true, &ip4, "http://127.0.0.1");
            variants(false, &ip4, "http://[::1]");
        }

        {
            let url = url("http://[::1]");
            let ip6 = CookieDomain::try_from(url.host().unwrap()).expect("unable to parse Ipv6");
            matches(false, &ip6, "data:nonrelative");
            variants(false, &ip6, "http://127.0.0.1");
            variants(true, &ip6, "http://[::1]");
        }
    }

    #[test]
    fn from_strs() {
        assert_eq!(CookieDomain::Empty,
                   CookieDomain::try_from("").expect("unable to parse domain"));
        assert_eq!(CookieDomain::Empty,
                   CookieDomain::try_from(".").expect("unable to parse domain"));
        assert_eq!(CookieDomain::Empty,
                   CookieDomain::try_from("..").expect("unable to parse domain"));
        assert_eq!(CookieDomain::Suffix(String::from("example.com")),
                   CookieDomain::try_from("example.com").expect("unable to parse domain"));
        assert_eq!(CookieDomain::Suffix(String::from("example.com")),
                   CookieDomain::try_from(".example.com").expect("unable to parse domain"));
        assert_eq!(CookieDomain::Suffix(String::from("example.com")),
                   CookieDomain::try_from("..example.com").expect("unable to parse domain"));
    }

    #[test]
    fn from_raw_cookie() {
        fn raw_cookie(s: &str) -> RawCookie {
            RawCookie::parse(s).unwrap()
        }
        assert_eq!(CookieDomain::NotPresent,
                   CookieDomain::try_from(&raw_cookie("cookie=value"))
                       .expect("unable to parse domain"));
        // cookie::Cookie handles this (cookie.domain == None)
        assert_eq!(CookieDomain::NotPresent,
                   CookieDomain::try_from(&raw_cookie("cookie=value; Domain="))
                       .expect("unable to parse domain"));
        // cookie::Cookie does not handle this (empty after stripping leading dot)
        assert_eq!(CookieDomain::Empty,
                   CookieDomain::try_from(&raw_cookie("cookie=value; Domain=."))
                       .expect("unable to parse domain"));
        assert_eq!(CookieDomain::Suffix(String::from("example.com")),
                   CookieDomain::try_from(&raw_cookie("cookie=value; Domain=.example.com"))
                       .expect("unable to parse domain"));
        assert_eq!(CookieDomain::Suffix(String::from("example.com")),
                   CookieDomain::try_from(&raw_cookie("cookie=value; Domain=example.com"))
                       .expect("unable to parse domain"));
    }

    #[test]
    fn matches_suffix() {
        {
            let suffix = CookieDomain::try_from("example.com").expect("unable to parse domain");
            variants(true, &suffix, "http://example.com");     //  exact match
            variants(true, &suffix, "http://foo.example.com"); //  suffix match
            variants(false, &suffix, "http://example.org");    //  no match
            variants(false, &suffix, "http://xample.com");     //  request is the suffix, no match
            variants(false, &suffix, "http://fooexample.com"); //  suffix, but no "." b/w foo and example, no match
        }

        {
            // strip leading dot
            let suffix = CookieDomain::try_from(".example.com").expect("unable to parse domain");
            variants(true, &suffix, "http://example.com");
            variants(true, &suffix, "http://foo.example.com");
            variants(false, &suffix, "http://example.org");
            variants(false, &suffix, "http://xample.com");
            variants(false, &suffix, "http://fooexample.com");
        }

        {
            // multiple leading dots are stripped
            let suffix = CookieDomain::try_from("..example.com").expect("unable to parse domain");
            variants(true, &suffix, "http://example.com");
            variants(true, &suffix, "http://foo.example.com");
            variants(false, &suffix, "http://example.org");
            variants(false, &suffix, "http://xample.com");
            variants(false, &suffix, "http://fooexample.com");
            variants(true, &suffix, "http://.example.com"); // Url::parse will parse this as "http://example.com"
        }

        {
            // an exact string match, although an IP is specified
            let suffix = CookieDomain::try_from("127.0.0.1").expect("unable to parse Ipv4");
            variants(true, &suffix, "http://127.0.0.1");
        }

        {
            // an exact string match, although an IP is specified
            let suffix = CookieDomain::try_from("[::1]").expect("unable to parse Ipv6");
            variants(true, &suffix, "http://[::1]");
        }

        {
            // non-identical suffix match only works for host names (i.e. not IPs)
            let suffix = CookieDomain::try_from("0.0.1").expect("unable to parse Ipv4");
            variants(false, &suffix, "http://127.0.0.1");
        }
    }
}

mod serde {
    #[cfg(test)]
    mod tests {
        use serde_json;
        use try_from::TryFrom;

        use cookie_domain::CookieDomain;
        use utils::test::*;

        fn encode_decode(cd: &CookieDomain, exp_json: &str) {
            let encoded = serde_json::to_string(cd).unwrap();
            assert!(exp_json == encoded,
                    "expected: '{}'\n encoded: '{}'",
                    exp_json,
                    encoded);
            let decoded: CookieDomain = serde_json::from_str(&encoded).unwrap();
            assert!(*cd == decoded,
                    "expected: '{:?}'\n decoded: '{:?}'",
                    cd,
                    decoded);
        }

        #[test]
        fn serde() {
            let url = url("http://example.com");
            encode_decode(&CookieDomain::try_from(url.host().unwrap())
                              .expect("cannot parse domain"),
                          "{\"HostOnly\":\"example.com\"}");
            encode_decode(&CookieDomain::try_from(".example.com").expect("cannot parse domain"),
                          "{\"Suffix\":\"example.com\"}");
            encode_decode(&CookieDomain::NotPresent, "\"NotPresent\"");
            encode_decode(&CookieDomain::Empty, "\"Empty\"");
        }
    }
}

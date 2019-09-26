= v0.8.0 =
* remove `failure` for Error handling

= v0.7.0 =
* Update to latest `cookie_store` and `cookie`
* Document `cookie_store` support availibility in `reqwest`

= v0.6.0 =
* Introduce features `default-tls` and `rustls-tls` to enable control of dependency (`reqwest`) features

= v0.6.5 =
* Bugfix for multiple Set-Cookie values

= v0.6.3 =
* Add `SessionClient::send()` fn, and simplify the `{get,post,...}_with` functions.
  * BREAKING: `*_with` fns now take a `prepare` `FnOnce` returning `Self::Request`,
    instead of `prepare_and_send` which returned `Result<Self::Response, Self::SendError>`
* Introduce convenience `get`, `post`, etc. methods
* BREAKING: Remove various `Deref` impls

= v0.6.0 =
Split `CookieStore` into separate [crate](https://crates.io/crates/cookie_store)

= v0.5.0 =
Refactor and reduce some trait and macro usage

= v0.4.0 =
* Update to Rust 2018 edition

= v0.3.1 =

* Upgrades to `cookies` v0.11
* Minor dependency upgrades

= v0.3 =

* Upgrades to `reqwest` v0.9
* Replaces `error-chain` with `failure`

= v0.2 =

* Removes separate `ReqwestSession::ErrorKind`. Added as variant `::ErrorKind::Reqwest` instead.

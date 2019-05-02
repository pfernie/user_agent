[![Build Status](https://travis-ci.org/pfernie/user_agent.svg?branch=master)](https://travis-ci.org/pfernie/user_agent)
[![Gitter chat](https://badges.gitter.im/gitterHQ/gitter.png)](https://gitter.im/user_agent)

[Documentation](https://docs.rs/user_agent/)

NOTE: `reqwest` provides support for a [cookie_store](https://docs.rs/reqwest/0.9.16/reqwest/struct.ClientBuilder.html#method.cookie_store) as of `v0.9.14`. It currently lacks an API for saving/loading a `CookieStore`, but consider using the directly provided functionality in lieu of this crate.

Provides the concept of a user agent session, storing and retrieving cookies over multiple HTTP requests (a `Session`).

Included is an implementation of `Session` using a [reqwest](https://crates.io/crates/reqwest) `reqwest::Client`.

The RFC6265 implementation has been moved to a separate [repo](https://github.com/pfernie/cookie_store)/[crate](https://crates.io/crates/cookie_store).

## License
This project is licensed and distributed under the terms of both the MIT license and Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT)

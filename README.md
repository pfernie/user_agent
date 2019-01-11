[![Build Status](https://travis-ci.org/pfernie/user_agent.svg?branch=master)](https://travis-ci.org/pfernie/user_agent)
[![Gitter chat](https://badges.gitter.im/gitterHQ/gitter.png)](https://gitter.im/user_agent)

Provides the concept of a user agent session, storing and retrieving cookies over multiple HTTP requests (a `Session`).

Included is an implementation of `Session` using a [reqwest](https://crates.io/crates/reqwest) `reqwest::Client`.

The RFC6265 implementation has been moved to a separate [repo](https://github.com/pfernie/cookie_store)/[crate](https://crates.io/crates/cookie_store).

## License
This project is licensed and distributed under the terms of both the MIT license and Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT)

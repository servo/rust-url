# Changelog

All notable changes to the url package will be documented in this file.

Changes to the other packages in this project can be found in their respective
directories:

  * [data-url](data-url/CHANGELOG.md)
  * [idna](idna/CHANGELOG.md)
  * [percent-encoding](percent_encoding/CHANGELOG.md)

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2019-07-23

### Changed

* The minimum supported Rust version is now v1.33.0 ([#510], [`9ab946f34`],
  and [#517]).

* Serde has been bumped to v1.x ([#512]).

* Exhaustive matching of `url::ParseError` and `url::SyntaxError` is now
  discouraged via a hidden enum variant ([#525]), so that adding additional
  variants in the future will not be considered a breaking change.

### Removed

* `url::Url` no longer implements `std::net::ToSocketAddrs` ([`6e0820148`]),
  and the related method `with_default_port` and type `HostAndPort` have been
  removed as well. The implementation of these features introduced a large
  amount of API surface area and was not considered to be a core competency of
  the project.

* The `idna` and `percent_export` crates are no longer exported by the `url`
  crate, so that breaking changes to those crates do not constitute a breaking
  change to the `url` crate ([`fe74a60bd`]).

* `_charset_` support ([`47e2286ff`]). TODO: say more.

* `rust-encoding` support ([`b567a51e7`]). TODO: say more.

* rustc-serialize is no longer supported. Specifically, the `rustc-serialize`
  feature has been removed, and so it is no longer possible to configure
  `url::Url` to implement the `rustc_serialize::Encodable` and
  `rustc_serialize::Decodable` traits ([#507]). rustc-serialize is deprecated,
  and Serde has been stable for over two years.

* The `heapsize` feature has been removed, as the [heapsize] project is no
  longer maintained ([`394e63a75`]).

* The `url_serde` crate is no longer maintained, as the `url` crate now ships
  support for Serde 1.x (when the `serde` feature is enabled) ([`51d6b33f7`]).

### Fixed

* Domains that have trailing hyphens are no longer incorrectly rejected
  ([#484]).

## 1.7.2 - 2018-07-06

The changelog was not maintained for v1.7.2 and earlier.

[Unreleased]: https://github.com/servo/rust-url/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/servo/rust-url/compare/v1.7.2...v2.0.0

[#484]: https://github.com/servo/rust-url/pull/484
[#507]: https://github.com/servo/rust-url/pull/507
[#510]: https://github.com/servo/rust-url/pull/510
[#512]: https://github.com/servo/rust-url/pull/512
[#517]: https://github.com/servo/rust-url/pull/517
[#525]: https://github.com/servo/rust-url/pull/525

[`394e63a75`]: https://github.com/servo/rust-url/commit/394e63a7518e1bfe8e106ebc7938706b10cfa1aa
[`47e2286ff`]: https://github.com/servo/rust-url/commit/47e2286ff32359879e69651409ed08385949eb8c
[`51d6b33f7`]: https://github.com/servo/rust-url/commit/51d6b33f717d29880cb53a1f5bf0d061d846ad35
[`6e0820148`]: https://github.com/servo/rust-url/commit/6e082014827061a79a27be1d7712e53a84c28280
[`9ab946f34`]: https://github.com/servo/rust-url/commit/9ab946f3419ed14142227e9e1dfea9bbb6ac5c17
[`b567a51e7`]: https://github.com/servo/rust-url/commit/b567a51e784bae1fdad1d1e5d7e4dcb00b406080
[`fe74a60bd`]: https://github.com/servo/rust-url/commit/fe74a60bd0636c5e5da920674b9bbffc22f3c384

[heapsize]: https://github.com/servo/heapsize
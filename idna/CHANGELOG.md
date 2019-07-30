# Changelog

All notable changes to the idna package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.2.0 - 2019-07-23

### Added

* Support for the `CheckHyphens` flag in the domain to ASCII algorithm ([#484]).

* A `domain_to_ascii_strict` function, which runs the domain to ASCII algorithm
  with the `beStrict` flag set ([`737128608`]).

### Changed

* The algorithms are now configured through a `Config` struct, which
  can be modified via the builder pattern, rather than by directly passing
  a flags struct to the implicated functions ([#484]).

### Removed

* The `uts46` module is private. The relevant structs within are re-exported at
  the top level ([`5aeaf89af`]).

## 0.1.5 - 2018-07-06

The changelog was not maintained for v0.1.5 and earlier.

[#484]: https://github.com/servo/rust-url/pull/484

[`5aeaf89af`]: https://github.com/servo/rust-url/commit/5aeaf89afe43c78eef7c958b1089bd586f68c271
[`737128608`]: https://github.com/servo/rust-url/commit/7371286087b32d358610df1ad3a3b1f55f6836df
# Changelog

All notable changes to the percent-encoding package will be documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 2.0.0 - 2019-07-23

### Changed

* Encoding sets are now values of a new `percent_encoding::AsciiSet` type,
  rather than any type that implements `percent_encoding::EncodeSet` ([#519]).
  This fixes a longstanding bug where `EncodeSet`s could not be boxed ([#388]).
  The `EncodeSet` trait has accordingly been removed.

* The prepackaged encoding sets, like `percent_encoding::QUERY_ENCODE_SET` and
  `percent_encoding::PATH_SEGMENT_ENCODE_SET`, have been removed ([#519]).
  Instead, read the specifications relevant to your domain and construct your
  own encoding sets by using the `percent_encoding::AsciiSet` builder methods
  on either of the base encoding sets, `percent_encoding::CONTROLS` or
  `percent_encoding::NON_ALPHANUMERIC`.

## 1.0.1 - 2017-11-11

The changelog was not maintained for v1.0.1 and earlier.

[#388]: https://github.com/servo/rust-url/issues/388
[#519]: https://github.com/servo/rust-url/pull/619

rust-url
========

Rust implementation of the [URL Standard](http://url.spec.whatwg.org/).

See [Rust bug #10707](https://github.com/mozilla/rust/issues/10707).

This depends on [rust-encoding](https://github.com/lifthrasiir/rust-encoding).


API
---

```rust
pub struct URL {
    scheme: String,
    scheme_data: SchemeData,
    query: Option<String>,  // See form_urlencoded::parse_str() to get name/value pairs.
    fragment: Option<String>,
}

pub enum SchemeData {
    RelativeSchemeData(SchemeRelativeURL),
    OtherSchemeData(String),  // data: URLs, mailto: URLs, etc.
}

pub struct SchemeRelativeURL {
    username: String,
    password: Option<String>,
    host: Host,
    port: String,
    path: Vec<String>,
}

pub enum Host {
    Domain(String),  // Can only be empty in the file scheme
    IPv6(IPv6Address)
}

pub struct IPv6Address {
    pieces: [u16, ..8]
}


pub type ParseResult<T> = Result<T, &'static str>;

impl URL {
    // base_url is used to resolve relative URLs.
    // Relative URLs without a base return an error.
    pub fn parse(input: &str, base_url: Option<&URL>) -> ParseResult<URL>
    pub fn serialize(&self) -> String
    pub fn serialize_no_fragment(&self) -> String
}

impl Host {
    pub fn parse(input: &str) -> ParseResult<Host>
    pub fn serialize(&self) -> String
}

impl IPv6Address {
    pub fn parse(input: &str) -> ParseResult<IPv6Address>
    pub fn serialize(&self) -> String
}


/// application/x-www-form-urlencoded
/// Converts between a query string and name/value pairs.
pub mod form_urlencoded {
    pub fn parse_str(input: &str) -> Vec<(String, String)>
    pub fn parse_bytes(input: &[u8], encoding_override: Option<encoding::EncodingRef>,
                       use_charset: bool, isindex: bool) -> Option<Vec<(String, String)>>
    pub fn serialize(pairs: Vec<(String, String)>],
                     encoding_override: Option<encoding::EncodingRef>) -> String
}
```


To do
-----

Not necessarily in the given order:

* Add proper documentation
* Add `data:` URL parsing.
* Port rust-http and Servo.
* Add [IDNA support](http://url.spec.whatwg.org/#idna).
  Non-ASCII domains are a parse error for now.
  [Punycode](http://tools.ietf.org/html/rfc3492) is done,
  [Nameprep](http://tools.ietf.org/html/rfc3491) is the other big part.
* Add lots of tests.
  Contribute them to [web-platform-tests](https://github.com/w3c/web-platform-tests/tree/master/url).
* Consider switching the spec from a state machine to functional style, like this code.

// Copyright 2013-2015 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*!

<a href="https://github.com/servo/rust-url"><img style="position: absolute; top: 0; left: 0; border: 0;" src="../github.png" alt="Fork me on GitHub"></a>
<style>.sidebar { margin-top: 53px }</style>

rust-url is an implementation of the [URL Standard](http://url.spec.whatwg.org/)
for the [Rust](http://rust-lang.org/) programming language.

It builds with [Cargo](http://crates.io/).
To use it in your project, add this to your `Cargo.toml` file:

```Cargo
[dependencies.url]
git = "https://github.com/servo/rust-url"
```

Supporting encodings other than UTF-8 in query strings is an optional feature
that requires [rust-encoding](https://github.com/lifthrasiir/rust-encoding)
and is off by default.
You can enable it with
[Cargo’s *features* mechanism](http://doc.crates.io/manifest.html#the-[features]-section):

```Cargo
[dependencies.url]
git = "https://github.com/servo/rust-url"
features = ["query_encoding"]
```

… or by passing `--cfg 'feature="query_encoding"'` to rustc.


# URL parsing and data structures

First, URL parsing may fail for various reasons and therefore returns a `Result`.

```
use url::{Url, ParseError};

assert!(Url::parse("http://[:::1]") == Err(ParseError::InvalidIpv6Address))
```

Let’s parse a valid URL and look at its components.

```
use url::{Url, Host};

let issue_list_url = Url::parse(
    "https://github.com/rust-lang/rust/issues?labels=E-easy&state=open"
).unwrap();


assert!(issue_list_url.scheme() == "https");
assert!(issue_list_url.username() == "");
assert!(issue_list_url.password() == None);
assert!(issue_list_url.host_str() == Some("github.com"));
assert!(issue_list_url.host() == Some(Host::Domain("github.com")));
assert!(issue_list_url.port() == None);
assert!(issue_list_url.path() == "/rust-lang/rust/issues");
assert!(issue_list_url.path_segments().map(|c| c.collect::<Vec<_>>()) ==
        Some(vec!["rust-lang", "rust", "issues"]));
assert!(issue_list_url.query() == Some("labels=E-easy&state=open"));
assert!(issue_list_url.fragment() == None);
assert!(!issue_list_url.non_relative());
```

Some URLs are said to be "non-relative":
they don’t have a username, password, host, or port,
and their "path" is an arbitrary string rather than slash-separated segments:

```
use url::Url;

let data_url = Url::parse("data:text/plain,Hello?World#").unwrap();

assert!(data_url.non_relative());
assert!(data_url.scheme() == "data");
assert!(data_url.path() == "text/plain,Hello");
assert!(data_url.path_segments().is_none());
assert!(data_url.query() == Some("World"));
assert!(data_url.fragment() == Some(""));
```


# Base URL

Many contexts allow URL *references* that can be relative to a *base URL*:

```html
<link rel="stylesheet" href="../main.css">
```

Since parsed URL are absolute, giving a base is required for parsing relative URLs:

```
use url::{Url, ParseError};

assert!(Url::parse("../main.css") == Err(ParseError::RelativeUrlWithoutBase))
```

Use the `join` method on an `Url` to use it as a base URL:

```
use url::Url;

let this_document = Url::parse("http://servo.github.io/rust-url/url/index.html").unwrap();
let css_url = this_document.join("../main.css").unwrap();
assert_eq!(css_url.as_str(), "http://servo.github.io/rust-url/main.css")
*/

#![cfg_attr(feature="heap_size", feature(plugin, custom_derive))]
#![cfg_attr(feature="heap_size", plugin(heapsize_plugin))]

#[cfg(feature="rustc-serialize")] extern crate rustc_serialize;
#[macro_use] extern crate matches;
#[cfg(feature="serde")] extern crate serde;
#[cfg(feature="heap_size")] #[macro_use] extern crate heapsize;

extern crate idna;

use host::HostInternal;
use percent_encoding::{PATH_SEGMENT_ENCODE_SET, percent_encode, percent_decode};
use std::cmp;
use std::fmt;
use std::hash;
use std::ops::{Range, RangeFrom, RangeTo};
use std::path::{Path, PathBuf};
use std::str;

pub use encoding::EncodingOverride;
pub use origin::Origin;
pub use host::Host;
pub use parser::ParseError;
pub use slicing::Position;
pub use webidl::WebIdl;

mod encoding;
mod host;
mod idna_mapping;
mod origin;
mod parser;
mod slicing;
mod webidl;

pub mod percent_encoding;
pub mod form_urlencoded;

/// A parsed URL record.
#[derive(Clone)]
#[cfg_attr(feature="heap_size", derive(HeapSizeOf))]
pub struct Url {
    serialization: String,
    non_relative: bool,

    // Components
    scheme_end: u32,  // Before ':'
    username_end: u32,  // Before ':' (if a password is given) or '@' (if not)
    host_start: u32,
    host_end: u32,
    host: HostInternal,
    port: Option<u16>,
    path_start: u32,  // Before initial '/' if !non_relative
    query_start: Option<u32>,  // Before '?', unlike Position::QueryStart
    fragment_start: Option<u32>,  // Before '#', unlike Position::FragmentStart
}

impl Url {
    /// Parse an absolute URL from a string.
    #[inline]
    pub fn parse(input: &str) -> Result<Url, ::ParseError> {
        Url::parse_with(input, None, EncodingOverride::utf8(), None)
    }

    /// Parse a string as an URL, with this URL as the base URL.
    #[inline]
    pub fn join(&self, input: &str) -> Result<Url, ::ParseError> {
        Url::parse_with(input, Some(self), EncodingOverride::utf8(), None)
    }

    /// The URL parser with all of its parameters.
    ///
    /// `encoding_override` is a legacy concept only relevant for HTML.
    /// When it’s not needed,
    /// `s.parse::<Url>()`, `Url::from_str(s)` and `url.join(s)` can be used instead.
    pub fn parse_with(input: &str,
                      base_url: Option<&Url>,
                      encoding_override: EncodingOverride,
                      log_syntax_violation: Option<&Fn(&'static str)>)
                      -> Result<Url, ::ParseError> {
        parser::Parser {
            serialization: String::with_capacity(input.len()),
            base_url: base_url,
            query_encoding_override: encoding_override,
            log_syntax_violation: log_syntax_violation,
            context: parser::Context::UrlParser,
        }.parse_url(input)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.serialization
    }

    /// Return the scheme of this URL, as an ASCII string without the ':' delimiter.
    #[inline]
    pub fn scheme(&self) -> &str {
        self.slice(..self.scheme_end)
    }

    /// Return whether this URL is non-relative (typical of e.g. `data:` and `mailto:` URLs.)
    #[inline]
    pub fn non_relative(&self) -> bool {
        self.non_relative
    }

    /// Return the username for this URL (typically the empty string)
    /// as a percent-encoded ASCII string.
    pub fn username(&self) -> &str {
        if self.slice(self.scheme_end..).starts_with("://") {
            self.slice(self.scheme_end + 3..self.username_end)
        } else {
            ""
        }
    }

    /// Return the password for this URL, if any, as a percent-encoded ASCII string.
    pub fn password(&self) -> Option<&str> {
        if self.byte_at(self.username_end) == b':' {
            debug_assert!(self.host().is_some());
            debug_assert!(self.byte_at(self.host_start - 1) == b'@');
            Some(self.slice(self.username_end + 1..self.host_start - 1))
        } else {
            None
        }
    }

    /// Return the string representation of the host (domain or IP address) for this URL, if any.
    /// Non-ASCII domains are punycode-encoded per IDNA.
    ///
    /// Non-relative URLs (typical of `data:` and `mailto:`) and some `file:` URLs
    /// don’t have a host.
    ///
    /// See also the `host` method.
    pub fn host_str(&self) -> Option<&str> {
        if matches!(self.host, HostInternal::None) {
            None
        } else {
            Some(self.slice(self.host_start..self.host_end))
        }
    }

    /// Return the parsed representation of the host for this URL.
    /// Non-ASCII domain labels are punycode-encoded per IDNA.
    ///
    /// Non-relative URLs (typical of `data:` and `mailto:`) and some `file:` URLs
    /// don’t have a host.
    ///
    /// See also the `host_str` method.
    pub fn host(&self) -> Option<Host<&str>> {
        match self.host {
            HostInternal::None => None,
            HostInternal::Domain => Some(Host::Domain(self.slice(self.host_start..self.host_end))),
            HostInternal::Ipv4(address) => Some(Host::Ipv4(address)),
            HostInternal::Ipv6(address) => Some(Host::Ipv6(address)),
        }
    }

    /// Return the port number for this URL, if any.
    #[inline]
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Return the port number for this URL, or the default port number if it is known.
    ///
    /// This method only knows the default port number
    /// of the `http`, `https`, `ws`, `wss`, `ftp`, and `gopher` schemes.
    ///
    /// For URLs in these schemes, this method always returns `Some(_)`.
    /// For other schemes, it is the same as `Url::port()`.
    #[inline]
    pub fn port_or_default(&self) -> Option<u16> {
        self.port.or_else(|| parser::default_port(self.scheme()))
    }

    /// Return the path for this URL, as a percent-encoded ASCII string.
    /// For relative URLs, this starts with a '/' slash
    /// and continues with slash-separated path segments.
    /// For non-relative URLs, this is an arbitrary string that doesn’t start with '/'.
    pub fn path(&self) -> &str {
        match (self.query_start, self.fragment_start) {
            (None, None) => self.slice(self.path_start..),
            (Some(next_component_start), _) |
            (None, Some(next_component_start)) => {
                self.slice(self.path_start..next_component_start)
            }
        }
    }

    /// If this URL is relative, return an iterator of '/' slash-separated path segments,
    /// each as a percent-encoded ASCII string.
    ///
    /// Return `None` for non-relative URLs, or an iterator of at least one string.
    pub fn path_segments(&self) -> Option<str::Split<char>> {
        if self.non_relative {
            None
        } else {
            let path = self.path();
            debug_assert!(path.starts_with("/"));
            Some(path[1..].split('/'))
        }
    }

    /// Return this URL’s query string, if any, as a percent-encoded ASCII string.
    pub fn query(&self) -> Option<&str> {
        match (self.query_start, self.fragment_start) {
            (None, _) => None,
            (Some(query_start), None) => {
                debug_assert!(self.byte_at(query_start) == b'?');
                Some(self.slice(query_start + 1..))
            }
            (Some(query_start), Some(fragment_start)) => {
                debug_assert!(self.byte_at(query_start) == b'?');
                Some(self.slice(query_start + 1..fragment_start))
            }
        }
    }

    /// Return this URL’s fragment identifier, if any.
    ///
    /// **Note:** the parser does *not* percent-encode this component,
    /// but the input may be percent-encoded already.
    pub fn fragment(&self) -> Option<&str> {
        self.fragment_start.map(|start| {
            debug_assert!(self.byte_at(start) == b'#');
            self.slice(start + 1..)
        })
    }

    /// Convert a file name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute or,
    /// on Windows, if the prefix is not a disk prefix (e.g. `C:`).
    pub fn from_file_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let mut serialization = "file://".to_owned();
        let path_start = serialization.len() as u32;
        try!(path_to_file_url_segments(path.as_ref(), &mut serialization));
        Ok(Url {
            serialization: serialization,
            non_relative: false,
            scheme_end: "file".len() as u32,
            username_end: path_start,
            host_start: path_start,
            host_end: path_start,
            host: HostInternal::None,
            port: None,
            path_start: path_start,
            query_start: None,
            fragment_start: None,
        })
    }

    /// Convert a directory name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute or,
    /// on Windows, if the prefix is not a disk prefix (e.g. `C:`).
    ///
    /// Compared to `from_file_path`, this ensure that URL’s the path has a trailing slash
    /// so that the entire path is considered when using this URL as a base URL.
    ///
    /// For example:
    ///
    /// * `"index.html"` parsed with `Url::from_directory_path(Path::new("/var/www"))`
    ///   as the base URL is `file:///var/www/index.html`
    /// * `"index.html"` parsed with `Url::from_file_path(Path::new("/var/www"))`
    ///   as the base URL is `file:///var/index.html`, which might not be what was intended.
    ///
    /// Note that `std::path` does not consider trailing slashes significant
    /// and usually does not include them (e.g. in `Path::parent()`).
    pub fn from_directory_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let mut url = try!(Url::from_file_path(path));
        if !url.serialization.ends_with('/') {
            url.serialization.push('/')
        }
        Ok(url)
    }

    /// Assuming the URL is in the `file` scheme or similar,
    /// convert its path to an absolute `std::path::Path`.
    ///
    /// **Note:** This does not actually check the URL’s `scheme`,
    /// and may give nonsensical results for other schemes.
    /// It is the user’s responsibility to check the URL’s scheme before calling this.
    ///
    /// ```
    /// # use url::Url;
    /// # let url = Url::parse("file:///etc/passwd").unwrap();
    /// let path = url.to_file_path();
    /// ```
    ///
    /// Returns `Err` if the host is neither empty nor `"localhost"`,
    /// or if `Path::new_opt()` returns `None`.
    /// (That is, if the percent-decoded path contains a NUL byte or,
    /// for a Windows path, is not UTF-8.)
    #[inline]
    pub fn to_file_path(&self) -> Result<PathBuf, ()> {
        // FIXME: Figure out what to do w.r.t host.
        if matches!(self.host(), None | Some(Host::Domain("localhost"))) {
            if let Some(segments) = self.path_segments() {
                return file_url_segments_to_pathbuf(segments)
            }
        }
        Err(())
    }

    /// Parse the URL’s query string, if any, as `application/x-www-form-urlencoded`
    /// and return a vector of (key, value) pairs.
    #[inline]
    pub fn query_pairs(&self) -> Option<Vec<(String, String)>> {
        self.query().map(|query| form_urlencoded::parse(query.as_bytes()))
    }

    // Private helper methods:

    #[inline]
    fn slice<R>(&self, range: R) -> &str where R: RangeArg {
        range.slice_of(&self.serialization)
    }

    #[inline]
    fn byte_at(&self, i: u32) -> u8 {
        self.serialization.as_bytes()[i as usize]
    }
}

/// Parse a string as an URL, without a base URL or encoding override.
impl str::FromStr for Url {
    type Err = ParseError;

    #[inline]
    fn from_str(input: &str) -> Result<Url, ::ParseError> {
        Url::parse(input)
    }
}

/// Display the serialization of this URL.
impl fmt::Display for Url {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.serialization, formatter)
    }
}

/// Debug the serialization of this URL.
impl fmt::Debug for Url {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.serialization, formatter)
    }
}

/// URLs compare like their serialization.
impl Eq for Url {}

/// URLs compare like their serialization.
impl PartialEq for Url {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.serialization == other.serialization
    }
}

/// URLs compare like their serialization.
impl Ord for Url {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.serialization.cmp(&other.serialization)
    }
}

/// URLs compare like their serialization.
impl PartialOrd for Url {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.serialization.partial_cmp(&other.serialization)
    }
}

/// URLs hash like their serialization.
impl hash::Hash for Url {
    #[inline]
    fn hash<H>(&self, state: &mut H) where H: hash::Hasher {
        hash::Hash::hash(&self.serialization, state)
    }
}

/// Return the serialization of this URL.
impl AsRef<str> for Url {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.serialization
    }
}

trait RangeArg {
    fn slice_of<'a>(&self, s: &'a str) -> &'a str;
}

impl RangeArg for Range<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[self.start as usize .. self.end as usize]
    }
}

impl RangeArg for RangeFrom<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[self.start as usize ..]
    }
}

impl RangeArg for RangeTo<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[.. self.end as usize]
    }
}

#[cfg(feature="rustc-serialize")]
impl rustc_serialize::Encodable for Url {
    fn encode<S: rustc_serialize::Encoder>(&self, encoder: &mut S) -> Result<(), S::Error> {
        encoder.emit_str(self.as_str())
    }
}


#[cfg(feature="rustc-serialize")]
impl rustc_serialize::Decodable for Url {
    fn decode<D: rustc_serialize::Decoder>(decoder: &mut D) -> Result<Url, D::Error> {
        Url::parse(&*try!(decoder.read_str())).map_err(|error| {
            decoder.error(&format!("URL parsing error: {}", error))
        })
    }
}

/// Serializes this URL into a `serde` stream.
///
/// This implementation is only available if the `serde` Cargo feature is enabled.
#[cfg(feature="serde")]
impl serde::Serialize for Url {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: serde::Serializer {
        format!("{}", self).serialize(serializer)
    }
}

/// Deserializes this URL from a `serde` stream.
///
/// This implementation is only available if the `serde` Cargo feature is enabled.
#[cfg(feature="serde")]
impl serde::Deserialize for Url {
    fn deserialize<D>(deserializer: &mut D) -> Result<Url, D::Error> where D: serde::Deserializer {
        let string_representation: String = try!(serde::Deserialize::deserialize(deserializer));
        Ok(Url::parse(&string_representation).unwrap())
    }
}

#[cfg(unix)]
fn path_to_file_url_segments(path: &Path, serialization: &mut String) -> Result<(), ()> {
    use std::os::unix::prelude::OsStrExt;
    if !path.is_absolute() {
        return Err(())
    }
    // skip the root component
    for component in path.components().skip(1) {
        serialization.push('/');
        serialization.extend(percent_encode(
            component.as_os_str().as_bytes(), PATH_SEGMENT_ENCODE_SET))
    }
    Ok(())
}

#[cfg(windows)]
fn path_to_file_url_segments(path: &Path, serialization: &mut String) -> Result<(), ()> {
    path_to_file_url_segments_windows(path, serialization)
}

// Build this unconditionally to alleviate https://github.com/servo/rust-url/issues/102
#[cfg_attr(not(windows), allow(dead_code))]
fn path_to_file_url_segments_windows(path: &Path, serialization: &mut String) -> Result<(), ()> {
    use std::path::{Prefix, Component};
    if !path.is_absolute() {
        return Err(())
    }
    let mut components = path.components();
    let disk = match components.next() {
        Some(Component::Prefix(ref p)) => match p.kind() {
            Prefix::Disk(byte) => byte,
            Prefix::VerbatimDisk(byte) => byte,
            _ => return Err(()),
        },

        // FIXME: do something with UNC and other prefixes?
        _ => return Err(())
    };

    // Start with the prefix, e.g. "C:"
    serialization.push('/');
    serialization.push(disk as char);
    serialization.push(':');

    for component in components {
        if component == Component::RootDir { continue }
        // FIXME: somehow work with non-unicode?
        let component = try!(component.as_os_str().to_str().ok_or(()));
        serialization.push('/');
        serialization.extend(percent_encode(component.as_bytes(), PATH_SEGMENT_ENCODE_SET));
    }
    Ok(())
}

#[cfg(unix)]
fn file_url_segments_to_pathbuf(segments: str::Split<char>) -> Result<PathBuf, ()> {
    use std::ffi::OsStr;
    use std::os::unix::prelude::OsStrExt;
    use std::path::PathBuf;

    let mut bytes = Vec::new();
    for segment in segments {
        bytes.push(b'/');
        bytes.extend(percent_decode(segment.as_bytes()));
    }
    let os_str = OsStr::from_bytes(&bytes);
    let path = PathBuf::from(os_str);
    debug_assert!(path.is_absolute(),
                  "to_file_path() failed to produce an absolute Path");
    Ok(path)
}

#[cfg(windows)]
fn file_url_segments_to_pathbuf(segments: str::Split<char>) -> Result<PathBuf, ()> {
    file_url_segments_to_pathbuf_windows(segments)
}

// Build this unconditionally to alleviate https://github.com/servo/rust-url/issues/102
#[cfg_attr(not(windows), allow(dead_code))]
fn file_url_segments_to_pathbuf_windows(mut segments: str::Split<char>) -> Result<PathBuf, ()> {
    let first = try!(segments.next().ok_or(()));
    if first.len() != 2 || !first.starts_with(parser::ascii_alpha)
            || first.as_bytes()[1] != b':' {
        return Err(())
    }
    let mut string = first.to_owned();
    for segment in segments {
        string.push('\\');

        // Currently non-unicode windows paths cannot be represented
        match String::from_utf8(percent_decode(segment.as_bytes()).collect()) {
            Ok(s) => string.push_str(&s),
            Err(..) => return Err(()),
        }
    }
    let path = PathBuf::from(string);
    debug_assert!(path.is_absolute(),
                  "to_file_path() failed to produce an absolute Path");
    Ok(path)
}

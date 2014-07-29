// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name = "url_"]
#![crate_type = "dylib"]
#![crate_type = "rlib"]

//! <a href="https://github.com/servo/rust-url"><img style="position: absolute; top: 0; left: 0; border: 0;" src="../github.png" alt="Fork me on GitHub"></a>
//! <style>.sidebar { display: none }</style>
//!
//! rust-url is an implementation of the [URL Standard](http://url.spec.whatwg.org/)
//! for the [Rust](http://rust-lang.org/) programming language.
//!
//! It builds with [Cargo](http://crates.io/).
//! To use it in your project, add this to your `Cargo.toml` file:
//!
//! ```Cargo
//! [dependencies.url]
//! git = "https://github.com/servo/rust-url"
//! ```
//!
//! This will automatically pull in the
//! [rust-encoding](https://github.com/lifthrasiir/rust-encoding) dependency.
//!
//! rust-url is a replacement of the [`url` crate](http://doc.rust-lang.org/url/index.html)
//! currently distributed with Rust.
//! rust-url’s crate is currently named `url_` with an underscore to avoid a naming conflict,
//! but the intent is to rename it to just `url` when the old crate eventually
//! [goes away](https://github.com/rust-lang/rust/issues/15874).
//! Therefore, it is recommended that you use this crate as follows:
//!
//! ```ignore
//! extern crate url = "url_";
//!
//! use url::{Url, ...};
//! ```
//!
//! That way, when the renaming is done, you will only need to change the `extern crate` line.
//!
//!
//! # URL parsing and data structures
//!
//! First, URL parsing may fail for various reasons and therefore returns a `Result`.
//!
//! ```
//! # use url_::Url;
//! assert!(Url::parse("http://[:::1]") == Err("Invalid IPv6 address"))
//! ```
//!
//! Let’s parse a valid URL and look at its components.
//!
//! ```
//! # use url_::{Url, RelativeSchemeData, NonRelativeSchemeData};
//! let issue_list_url = Url::parse(
//!     "https://github.com/rust-lang/rust/issues?labels=E-easy&state=open"
//! ).unwrap();
//!
//!
//! assert!(issue_list_url.scheme == "https".to_string());
//! assert!(issue_list_url.domain() == Some("github.com"));
//! assert!(issue_list_url.port() == Some(""));
//! assert!(issue_list_url.path() == Some(&["rust-lang".to_string(),
//!                                         "rust".to_string(),
//!                                         "issues".to_string()]));
//! assert!(issue_list_url.query == Some("labels=E-easy&state=open".to_string()));
//! assert!(issue_list_url.fragment == None);
//! match issue_list_url.scheme_data {
//!     RelativeSchemeData(..) => {},  // Expected
//!     NonRelativeSchemeData(..) => fail!(),
//! }
//! ```
//!
//! The `scheme`, `query`, and `fragment` are directly fields of the `Url` struct:
//! they apply to all URLs.
//! Every other components has accessors because they only apply to URLs said to be
//! “in a relative scheme”. `https` is a relative scheme, but `data` is not:
//!
//! ```
//! # use url_::{Url, NonRelativeSchemeData};
//! let data_url = Url::parse("data:text/plain,Hello#").unwrap();
//!
//! assert!(data_url.scheme == "data".to_string());
//! assert!(data_url.scheme_data == NonRelativeSchemeData("text/plain,Hello".to_string()));
//! assert!(data_url.non_relative_scheme_data() == Some("text/plain,Hello"));
//! assert!(data_url.query == None);
//! assert!(data_url.fragment == Some("".to_string()));
//! ```
//!
//!
//! # Base URL
//!
//! Many contexts allow URL *references* that can be relative to a *base URL*:
//!
//! ```html
//! <link rel="stylesheet" href="../main.css">
//! ```
//!
//! Since parsed URL are absolute, giving a base is required:
//!
//! ```
//! # use url_::Url;
//! assert!(Url::parse("../main.css") == Err("Relative URL without a base"))
//! ```
//!
//! `UrlParser` is a method-chaining API to provide various optional parameters
//! to URL parsing, including a base URL.
//!
//! ```
//! # use url_::{Url, UrlParser};
//! let this_document = Url::parse("http://servo.github.io/rust-url/url/index.html").unwrap();
//! let css_url = UrlParser::new().base_url(&this_document).parse("../main.css").unwrap();
//! assert!(css_url.serialize() == "http://servo.github.io/rust-url/main.css".to_string());
//! ```


#![feature(macro_rules, default_type_params)]

extern crate encoding;

#[cfg(test)]
extern crate serialize;

use std::cmp;
use std::fmt::{Formatter, FormatError, Show};
use std::hash;
use std::path::Path;
use std::ascii::OwnedStrAsciiExt;

use encoding::EncodingRef;


mod encode_sets;
mod parser;
pub mod form_urlencoded;
pub mod punycode;

#[cfg(test)]
mod tests;


/// The parsed representation of an absolute URL.
#[deriving(PartialEq, Eq, Clone)]
pub struct Url {
    /// The scheme (a.k.a. protocol) of the URL, in ASCII lower case.
    pub scheme: String,

    /// The components of the URL whose representation depends on where the scheme is *relative*.
    pub scheme_data: SchemeData,

    /// The query string of the URL.
    ///
    /// `None` if the `?` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, pecent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `query_pairs`, `set_query_from_pairs`,
    /// and `lossy_precent_decode_query` methods.
    pub query: Option<String>,

    /// The fragment identifier of the URL.
    ///
    /// `None` if the `#` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, pecent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_precent_decode_fragment` method.
    pub fragment: Option<String>,
}

/// The components of the URL whose representation depends on where the scheme is *relative*.
#[deriving(PartialEq, Eq, Clone)]
pub enum SchemeData {
    /// Components for URLs in a *relative* scheme such as HTTP.
    RelativeSchemeData(RelativeSchemeData),

    /// No further structure is assumed for *non-relative* schemes such as `data` and `mailto`.
    ///
    /// This is a single percent-encoded string, whose interpretation depends on the scheme.
    ///
    /// Percent encoded strings are within the ASCII range.
    NonRelativeSchemeData(String),
}

/// Components for URLs in a *relative* scheme such as HTTP.
#[deriving(PartialEq, Eq, Clone)]
pub struct RelativeSchemeData {
    /// The username of the URL, as a possibly empty, pecent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_precent_decode_username` method.
    pub username: String,

    /// The password of the URL.
    ///
    /// `None` if the `:` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, pecent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_precent_decode_password` method.
    pub password: Option<String>,

    /// The host of the URL, either a domain name or an IPv4 address
    pub host: Host,

    /// The port number of the URL, in ASCII decimal,
    /// or the empty string for no port number (in the file scheme) or the default port number.
    pub port: String,

    /// The path of the URL, as vector of pecent-encoded strings.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `serialize_path` method and,
    /// for URLs in the `file` scheme, the `to_file_path` method.
    pub path: Vec<String>,
}


/// The host name of an URL.
#[deriving(PartialEq, Eq, Clone)]
pub enum Host {
    /// A (DNS) domain name or an IPv4 address.
    ///
    /// FIXME: IPv4 probably should be a separate variant.
    /// See https://www.w3.org/Bugs/Public/show_bug.cgi?id=26431
    Domain(String),

    /// An IPv6 address, represented inside `[...]` square brackets
    /// so that `:` colon characters in the address are not ambiguous
    /// with the port number delimiter.
    Ipv6(Ipv6Address),
}


/// A 128 bit IPv6 address
pub struct Ipv6Address {
    pub pieces: [u16, ..8]
}

impl Clone for Ipv6Address {
    fn clone(&self) -> Ipv6Address {
        Ipv6Address { pieces: self.pieces }
    }
}

impl Eq for Ipv6Address {}

impl PartialEq for Ipv6Address {
    fn eq(&self, other: &Ipv6Address) -> bool {
        self.pieces == other.pieces
    }
}

impl<S: hash::Writer> hash::Hash<S> for Url {
    fn hash(&self, state: &mut S) {
        self.serialize().hash(state)
    }
}


/// A set of optional parameters for URL parsing.
pub struct UrlParser<'a> {
    base_url: Option<&'a Url>,
    query_encoding_override: Option<EncodingRef>,
    error_handler: ErrorHandler,
    scheme_type_mapper: fn(scheme: &str) -> SchemeType,
}


/// A method-chaining API to provide a set of optional parameters for URL parsing.
impl<'a> UrlParser<'a> {
    /// Return a new UrlParser with default parameters.
    #[inline]
    pub fn new() -> UrlParser<'a> {
        UrlParser {
            base_url: None,
            query_encoding_override: None,
            error_handler: silent_handler,
            scheme_type_mapper: whatwg_scheme_type_mapper,
        }
    }

    /// Set the base URL used for resolving relative URL references, and return the `UrlParser`.
    /// The default is no base URL, so that relative URLs references fail to parse.
    #[inline]
    pub fn base_url<'b>(&'b mut self, value: &'a Url) -> &'b mut UrlParser<'a> {
        self.base_url = Some(value);
        self
    }

    /// Set the character encoding the query string is encoded as before percent-encoding,
    /// and return the `UrlParser`.
    ///
    /// This legacy quirk is only relevant to HTML.
    #[inline]
    pub fn query_encoding_override<'b>(&'b mut self, value: EncodingRef) -> &'b mut UrlParser<'a> {
        self.query_encoding_override = Some(value);
        self
    }

    /// Set an error handler for non-fatal parse errors, and return the `UrlParser`.
    ///
    /// Non-fatal parse errors are normally ignored by the parser,
    /// but indicate violations of authoring requirements.
    /// An error handler can be used, for example, to log these errors in the console
    /// of a browser’s developer tools.
    ///
    /// The error handler can choose to make the error fatal by returning `Err(..)`
    #[inline]
    pub fn error_handler<'b>(&'b mut self, value: ErrorHandler) -> &'b mut UrlParser<'a> {
        self.error_handler = value;
        self
    }

    /// Set a *scheme type mapper*, and return the `UrlParser`.
    ///
    /// The URL parser behaves differently based on the `SchemeType` of the URL.
    /// See the documentation for `SchemeType` for more details.
    /// A *scheme type mapper* returns a `SchemeType`
    /// based on the scheme as an ASCII lower case string,
    /// as found in the `scheme` field of an `Url` struct.
    ///
    /// The default scheme type mapper is as follows:
    ///
    /// ```ignore
    /// match scheme {
    ///     "file" => FileLikeRelativeScheme,
    ///     "ftp" => RelativeScheme("21"),
    ///     "gopher" => RelativeScheme("70"),
    ///     "http" => RelativeScheme("80"),
    ///     "https" => RelativeScheme("443"),
    ///     "ws" => RelativeScheme("80"),
    ///     "wss" => RelativeScheme("443"),
    ///     _ => NonRelativeScheme,
    /// }
    /// ```
    ///
    /// Note that unknown schemes default to non-relative.
    /// Overriding the scheme type mapper can allow, for example,
    /// parsing URLs in the `git` or `irc` scheme as relative.
    #[inline]
    pub fn scheme_type_mapper<'b>(&'b mut self, value: fn(scheme: &str) -> SchemeType)
                       -> &'b mut UrlParser<'a> {
        self.scheme_type_mapper = value;
        self
    }

    /// Parse `input` as an URL, with all the parameters previously set in the `UrlParser`.
    #[inline]
    pub fn parse(&self, input: &str) -> ParseResult<Url> {
        parser::parse_url(input, self)
    }
}


/// Private convenience methods for use in parser.rs
impl<'a> UrlParser<'a> {
    #[inline]
    fn parse_error(&self, message: &'static str) -> ParseResult<()> {
        (self.error_handler)(message)
    }

    #[inline]
    fn get_scheme_type(&self, scheme: &str) -> SchemeType {
        (self.scheme_type_mapper)(scheme)
    }
}


/// Determines the behavior of the URL parser for a given scheme.
#[deriving(PartialEq, Eq)]
pub enum SchemeType {
    /// Indicate that the scheme is *non-relative*.
    ///
    /// The *scheme data* of the URL
    /// (everything other than the scheme, query string, and fragment identifier)
    /// is parsed as a single percent-encoded string of which no structure is assumed.
    /// That string may need to be parsed further, per a scheme-specific format.
    NonRelativeScheme,

    /// Indicate that the scheme is *relative*, and what the default port number is.
    ///
    /// The *scheme data* is structured as
    /// *username*, *password*, *host*, *port number*, and *path*.
    /// Relative URL references are supported, if a base URL was given.
    /// The string value indicates the default port number as a string of ASCII digits,
    /// or the empty string to indicate no default port number.
    RelativeScheme(&'static str),

    /// Indicate a *relative* scheme similar to the *file* scheme.
    ///
    /// For example, you might want to have distinct `git+file` and `hg+file` URL schemes.
    ///
    /// This is like `RelativeScheme` except the host can be empty, there is no port number,
    /// and path parsing has (platform-independent) quirks to support Windows filenames.
    FileLikeRelativeScheme,
}

/// http://url.spec.whatwg.org/#relative-scheme
fn whatwg_scheme_type_mapper(scheme: &str) -> SchemeType {
    match scheme {
        "file" => FileLikeRelativeScheme,
        "ftp" => RelativeScheme("21"),
        "gopher" => RelativeScheme("70"),
        "http" => RelativeScheme("80"),
        "https" => RelativeScheme("443"),
        "ws" => RelativeScheme("80"),
        "wss" => RelativeScheme("443"),
        _ => NonRelativeScheme,
    }
}


pub type ParseResult<T> = Result<T, &'static str>;

/// This is called on non-fatal parse errors.
///
/// The handler can choose to continue or abort parsing by returning Ok() or Err(), respectively.
/// See the `UrlParser::error_handler` method.
///
/// FIXME: make this a by-ref closure when that’s supported.
pub type ErrorHandler = fn(reason: &'static str) -> ParseResult<()>;

fn silent_handler(_reason: &'static str) -> ParseResult<()> {
    Ok(())
}


impl Url {
    /// Parse an URL with the default `UrlParser` parameters.
    ///
    /// In particular, relative URL references are parse errors since no base URL is provided.
    #[inline]
    pub fn parse(input: &str) -> ParseResult<Url> {
        UrlParser::new().parse(input)
    }

    /// Convert a file name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute.
    ///
    /// This is Unix-only for now. FIXME: Figure out what to do on Windows.
    #[cfg(unix)]
    pub fn from_file_path(path: &Path) -> Result<Url, ()> {
        let path = try!(encode_file_path(path));
        Ok(Url::from_path_common(path))
    }

    /// Convert a directory name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute.
    ///
    /// Compared to `from_file_path`, this adds an empty component to the path
    /// (or, in terms of URL syntax, adds a trailing slash)
    /// so that the entire path is considered when using this URL as a base URL.
    ///
    /// For example:
    ///
    /// * `"index.html"` parsed with `Url::from_directory_path(Path::new("/var/www"))`
    ///   as the base URL is `file:///var/www/index.html`
    /// * `"index.html"` parsed with `Url::from_file_path(Path::new("/var/www/"))`
    ///   as the base URL is `file:///var/index.html`, which might not be what was intended.
    ///
    /// (Note that `Path::new` removes any trailing slash.)
    ///
    /// This is Unix-only for now. FIXME: Figure out what to do on Windows.
    #[cfg(unix)]
    pub fn from_directory_path(path: &Path) -> Result<Url, ()> {
        let mut path = try!(encode_file_path(path));
        // Add an empty path component (i.e. a trailing slash in serialization)
        // so that the entire path is used as a base URL.
        path.push("".to_string());
        Ok(Url::from_path_common(path))
    }

    fn from_path_common(path: Vec<String>) -> Url {
        Url {
            scheme: "file".to_string(),
            scheme_data: RelativeSchemeData(RelativeSchemeData {
                username: "".to_string(),
                password: None,
                port: "".to_string(),
                host: Domain("".to_string()),
                path: path,
            }),
            query: None,
            fragment: None,
        }
    }

    /// Assuming the URL is in the `file` scheme or similar,
    /// convert its path to an absolute `std::path::Path`.
    ///
    /// **Note:** This does not actually check the URL’s `scheme`,
    /// and may give nonsensical results for other schemes.
    /// It is the user’s responsibility to check the URL’s scheme before calling this.
    ///
    /// Returns `Err` if the URL is *non-relative*,
    /// or if its host is neither empty nor `"localhost"`.
    ///
    /// This is Unix-only for now. FIXME: Figure out what to do on Windows.
    #[inline]
    #[cfg(unix)]
    pub fn to_file_path(&self) -> Result<Path, ()> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => scheme_data.to_file_path(),
            NonRelativeSchemeData(..) => Err(()),
        }
    }

    /// Return the serialization of this URL as a string.
    pub fn serialize(&self) -> String {
        self.to_string()
    }

    /// Return the serialization of this URL, without the fragment identifier, as a string
    pub fn serialize_no_fragment(&self) -> String {
        UrlNoFragmentFormatter{ url: self }.to_string()
    }

    /// If the URL is *non-relative*, return the string scheme data.
    #[inline]
    pub fn non_relative_scheme_data<'a>(&'a self) -> Option<&'a str> {
        match self.scheme_data {
            RelativeSchemeData(..) => None,
            NonRelativeSchemeData(ref scheme_data) => Some(scheme_data.as_slice()),
        }
    }

    /// If the URL is in a *relative scheme*, return the structured scheme data.
    #[inline]
    pub fn relative_scheme_data<'a>(&'a self) -> Option<&'a RelativeSchemeData> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(scheme_data),
            NonRelativeSchemeData(..) => None,
        }
    }

    /// If the URL is in a *relative scheme*, return its username.
    #[inline]
    pub fn username<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.username.as_slice())
    }

    /// Percent-decode the URL’s username, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_precent_decode_username(&self) -> Option<String> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.lossy_precent_decode_username())
    }

    /// If the URL is in a *relative scheme*, return its password, if any.
    #[inline]
    pub fn password<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().and_then(|scheme_data|
            scheme_data.password.as_ref().map(|password| password.as_slice()))
    }

    /// Percent-decode the URL’s password, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_precent_decode_password(&self) -> Option<String> {
        self.relative_scheme_data().and_then(|scheme_data|
            scheme_data.lossy_precent_decode_password())
    }

    /// If the URL is in a *relative scheme*, return its structured host.
    #[inline]
    pub fn host<'a>(&'a self) -> Option<&'a Host> {
        self.relative_scheme_data().map(|scheme_data| &scheme_data.host)
    }

    /// If the URL is in a *relative scheme* and its host is a domain,
    /// return the domain as a string.
    #[inline]
    pub fn domain<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().and_then(|scheme_data| scheme_data.domain())
    }

    /// If the URL is in a *relative scheme*, serialize its host as a string.
    ///
    /// A domain a returned as-is, an IPv6 address between [] square brackets.
    #[inline]
    pub fn serialize_host(&self) -> Option<String> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.host.serialize())
    }

    /// If the URL is in a *relative scheme*, return its port.
    #[inline]
    pub fn port<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.port.as_slice())
    }

    /// If the URL is in a *relative scheme*, return its path components.
    #[inline]
    pub fn path<'a>(&'a self) -> Option<&'a [String]> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.path.as_slice())
    }

    /// If the URL is in a *relative scheme*, serialize its path as a string.
    ///
    /// The returned string starts with a "/" slash, and components are separated by slashes.
    /// A trailing slash represents an empty last component.
    #[inline]
    pub fn serialize_path(&self) -> Option<String> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.serialize_path())
    }

    /// Parse the URL’s query string, if any, as `application/x-www-form-urlencoded`
    /// and return a vector of (key, value) pairs.
    #[inline]
    pub fn query_pairs(&self) -> Option<Vec<(String, String)>> {
        self.query.as_ref().map(|query| form_urlencoded::parse_str(query.as_slice()))
    }

    /// Serialize an iterator of (key, value) pairs as `application/x-www-form-urlencoded`
    /// and set it as the URL’s query string.
    #[inline]
    pub fn set_query_from_pairs<'a, I: Iterator<(&'a str, &'a str)>>(&mut self, pairs: I) {
        self.query = Some(form_urlencoded::serialize(pairs, None));
    }

    /// Percent-decode the URL’s query string, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_precent_decode_query(&self) -> Option<String> {
        self.query.as_ref().map(|value| lossy_utf8_percent_decode(value.as_bytes()))
    }

    /// Percent-decode the URL’s fragment identifier, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_precent_decode_fragment(&self) -> Option<String> {
        self.fragment.as_ref().map(|value| lossy_utf8_percent_decode(value.as_bytes()))
    }
}


impl Show for Url {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        try!(UrlNoFragmentFormatter{ url: self }.fmt(formatter));
        match self.fragment {
            None => (),
            Some(ref fragment) => {
                try!(formatter.write(b"#"));
                try!(formatter.write(fragment.as_bytes()));
            }
        }
        Ok(())
    }
}

struct UrlNoFragmentFormatter<'a> {
    url: &'a Url
}

impl<'a> Show for UrlNoFragmentFormatter<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        try!(formatter.write(self.url.scheme.as_bytes()));
        try!(formatter.write(b":"));
        try!(self.url.scheme_data.fmt(formatter));
        match self.url.query {
            None => (),
            Some(ref query) => {
                try!(formatter.write(b"?"));
                try!(formatter.write(query.as_bytes()));
            }
        }
        Ok(())
    }
}


impl Show for SchemeData {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        match *self {
            RelativeSchemeData(ref scheme_data) => scheme_data.fmt(formatter),
            NonRelativeSchemeData(ref scheme_data) => scheme_data.fmt(formatter),
        }
    }
}


impl RelativeSchemeData {
    /// Percent-decode the URL’s username.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_precent_decode_username(&self) -> String {
        lossy_utf8_percent_decode(self.username.as_bytes())
    }

    /// Percent-decode the URL’s password, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_precent_decode_password(&self) -> Option<String> {
        self.password.as_ref().map(|value| lossy_utf8_percent_decode(value.as_bytes()))
    }

    /// Assuming the URL is in the `file` scheme or similar,
    /// convert its path to an absolute `std::path::Path`.
    ///
    /// **Note:** This does not actually check the URL’s `scheme`,
    /// and may give nonsensical results for other schemes.
    /// It is the user’s responsibility to check the URL’s scheme before calling this.
    ///
    /// Returns `Err` if the host is neither empty nor `"localhost"`.
    ///
    /// This is Unix-only for now. FIXME: Figure out what to do on Windows.
    #[cfg(unix)]
    pub fn to_file_path(&self) -> Result<Path, ()> {
        // FIXME: Figure out what to do w.r.t host.
        match self.domain() {
            Some("") | Some("localhost") => {
                if self.path.is_empty() {
                    Ok(Path::new("/"))
                } else {
                    let mut bytes = Vec::new();
                    for path_part in self.path.iter() {
                        bytes.push(b'/');
                        percent_decode_to(path_part.as_bytes(), &mut bytes);
                    }
                    let path = Path::new(bytes);
                    debug_assert!(path.is_absolute(),
                                  "to_file_path() failed to produce an absolute Path")
                    Ok(path)
                }
            }
            _ => Err(())
        }
    }

    /// If the host is a domain, return the domain as a string.
    #[inline]
    pub fn domain<'a>(&'a self) -> Option<&'a str> {
        match self.host {
            Domain(ref domain) => Some(domain.as_slice()),
            _ => None,
        }
    }

    /// Serialize the path as a string.
    ///
    /// The returned string starts with a "/" slash, and components are separated by slashes.
    /// A trailing slash represents an empty last component.
    pub fn serialize_path(&self) -> String {
        PathFormatter { path: &self.path }.to_string()
    }
}

struct PathFormatter<'a> {
    path: &'a Vec<String>
}

impl<'a> Show for PathFormatter<'a> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        if self.path.is_empty() {
            formatter.write(b"/")
        } else {
            for path_part in self.path.iter() {
                try!(formatter.write(b"/"));
                try!(formatter.write(path_part.as_bytes()));
            }
            Ok(())
        }
    }
}

impl Show for RelativeSchemeData {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        try!(formatter.write(b"//"));
        if !self.username.is_empty() || self.password.is_some() {
            try!(formatter.write(self.username.as_bytes()));
            match self.password {
                None => (),
                Some(ref password) => {
                    try!(formatter.write(b":"));
                    try!(formatter.write(password.as_bytes()));
                }
            }
            try!(formatter.write(b"@"));
        }
        try!(self.host.fmt(formatter));
        if !self.port.is_empty() {
            try!(formatter.write(b":"));
            try!(formatter.write(self.port.as_bytes()));
        }
        PathFormatter { path: &self.path }.fmt(formatter)
    }
}


#[allow(dead_code)]
struct UrlUtilsWrapper<'a> {
    url: &'a mut Url,
    parser: &'a UrlParser<'a>,
}


/// These methods are not meant for use in Rust code,
/// only to help implement the JavaScript URLUtils API: http://url.spec.whatwg.org/#urlutils
#[doc(hidden)]
trait UrlUtils {
    fn set_scheme(&mut self, input: &str) -> ParseResult<()>;
    fn set_username(&mut self, input: &str) -> ParseResult<()>;
    fn set_password(&mut self, input: &str) -> ParseResult<()>;
    fn set_host_and_port(&mut self, input: &str) -> ParseResult<()>;
    fn set_host(&mut self, input: &str) -> ParseResult<()>;
    fn set_port(&mut self, input: &str) -> ParseResult<()>;
    fn set_path(&mut self, input: &str) -> ParseResult<()>;
    fn set_query(&mut self, input: &str) -> ParseResult<()>;
    fn set_fragment(&mut self, input: &str) -> ParseResult<()>;
}

impl<'a> UrlUtils for UrlUtilsWrapper<'a> {
    /// `URLUtils.protocol` setter
    fn set_scheme(&mut self, input: &str) -> ParseResult<()> {
        match parser::parse_scheme(input.as_slice(), parser::SetterContext) {
            Some((scheme, _)) => {
                self.url.scheme = scheme;
                Ok(())
            },
            None => Err("Invalid scheme"),
        }
    }

    /// `URLUtils.username` setter
    fn set_username(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            RelativeSchemeData(RelativeSchemeData { ref mut username, .. }) => {
                username.truncate(0);
                utf8_percent_encode_to(input, USERNAME_ENCODE_SET, username);
                Ok(())
            },
            NonRelativeSchemeData(_) => Err("Can not set username on non-relative URL.")
        }
    }

    /// `URLUtils.password` setter
    fn set_password(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            RelativeSchemeData(RelativeSchemeData { ref mut password, .. }) => {
                let mut new_password = String::new();
                utf8_percent_encode_to(input, PASSWORD_ENCODE_SET, &mut new_password);
                *password = Some(new_password);
                Ok(())
            },
            NonRelativeSchemeData(_) => Err("Can not set password on non-relative URL.")
        }
    }

    /// `URLUtils.host` setter
    fn set_host_and_port(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            RelativeSchemeData(RelativeSchemeData { ref mut host, ref mut port, .. }) => {
                let scheme_type = self.parser.get_scheme_type(self.url.scheme.as_slice());
                let (new_host, new_port, _) = try!(parser::parse_host(
                    input, scheme_type, self.parser));
                *host = new_host;
                *port = new_port;
                Ok(())
            },
            NonRelativeSchemeData(_) => Err("Can not set host/port on non-relative URL.")
        }
    }

    /// `URLUtils.hostname` setter
    fn set_host(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            RelativeSchemeData(RelativeSchemeData { ref mut host, .. }) => {
                let (new_host, _) = try!(parser::parse_hostname(input, self.parser));
                *host = new_host;
                Ok(())
            },
            NonRelativeSchemeData(_) => Err("Can not set host on non-relative URL.")
        }
    }

    /// `URLUtils.port` setter
    fn set_port(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            RelativeSchemeData(RelativeSchemeData { ref mut port, .. }) => {
                let scheme_type = self.parser.get_scheme_type(self.url.scheme.as_slice());
                if scheme_type == FileLikeRelativeScheme {
                    return Err("Can not set port on file: URL.")
                }
                let (new_port, _) = try!(parser::parse_port(input, scheme_type, self.parser));
                *port = new_port;
                Ok(())
            },
            NonRelativeSchemeData(_) => Err("Can not set port on non-relative URL.")
        }
    }

    /// `URLUtils.pathname` setter
    fn set_path(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            RelativeSchemeData(RelativeSchemeData { ref mut path, .. }) => {
                let scheme_type = self.parser.get_scheme_type(self.url.scheme.as_slice());
                let (new_path, _) = try!(parser::parse_path_start(
                    input, parser::SetterContext, scheme_type, self.parser));
                *path = new_path;
                Ok(())
            },
            NonRelativeSchemeData(_) => Err("Can not set path on non-relative URL.")
        }
    }

    /// `URLUtils.search` setter
    fn set_query(&mut self, input: &str) -> ParseResult<()> {
        self.url.query = if input.is_empty() {
            None
        } else {
            let input = if input.starts_with("?") { input.slice_from(1) } else { input };
            let (new_query, _) = try!(parser::parse_query(
                input, parser::SetterContext, self.parser));
            Some(new_query)
        };
        Ok(())
    }

    /// `URLUtils.hash` setter
    fn set_fragment(&mut self, input: &str) -> ParseResult<()> {
        if self.url.scheme.as_slice() == "javascript" {
            return Err("Can not set fragment on a javascript: URL.")
        }
        self.url.fragment = if input.is_empty() {
            None
        } else {
            let input = if input.starts_with("#") { input.slice_from(1) } else { input };
            Some(try!(parser::parse_fragment(input, self.parser)))
        };
        Ok(())
    }
}


impl Host {
    /// Parse a host: either an IPv6 address in [] square brackets, or a domain.
    ///
    /// Returns `Err` for an empty host, an invalid IPv6 address,
    /// or a or invalid non-ASCII domain.
    ///
    /// FIXME: Add IDNA support for non-ASCII domains.
    pub fn parse(input: &str) -> ParseResult<Host> {
        if input.len() == 0 {
            Err("Empty host")
        } else if input.starts_with("[") {
            if input.ends_with("]") {
                Ipv6Address::parse(input.slice(1, input.len() - 1)).map(Ipv6)
            } else {
                Err("Invalid Ipv6 address")
            }
        } else {
            let decoded = percent_decode(input.as_bytes());
            let domain = String::from_utf8_lossy(decoded.as_slice());
            // TODO: Remove this check and use IDNA "domain to ASCII"
            if !domain.as_slice().is_ascii() {
                Err("Non-ASCII domains (IDNA) are not supported yet.")
            } else if domain.as_slice().find(&[
                '\0', '\t', '\n', '\r', ' ', '#', '%', '/', ':', '?', '@', '[', '\\', ']'
            ]).is_some() {
                Err("Invalid domain character.")
            } else {
                Ok(Domain(domain.into_string().into_ascii_lower()))
            }
        }
    }

    /// Serialize the host as a string.
    ///
    /// A domain a returned as-is, an IPv6 address between [] square brackets.
    pub fn serialize(&self) -> String {
        self.to_string()
    }
}


impl Show for Host {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        match *self {
            Domain(ref domain) => domain.fmt(formatter),
            Ipv6(ref address) => {
                try!(formatter.write(b"["));
                try!(address.fmt(formatter));
                formatter.write(b"]")
            }
        }
    }
}


impl Ipv6Address {
    /// Parse an IPv6 address, without the [] square brackets.
    pub fn parse(input: &str) -> ParseResult<Ipv6Address> {
        let input = input.as_bytes();
        let len = input.len();
        let mut is_ip_v4 = false;
        let mut pieces = [0, 0, 0, 0, 0, 0, 0, 0];
        let mut piece_pointer = 0u;
        let mut compress_pointer = None;
        let mut i = 0u;
        if input[0] == b':' {
            if input[1] != b':' {
                return Err("Invalid IPv6 address")
            }
            i = 2;
            piece_pointer = 1;
            compress_pointer = Some(1u);
        }

        while i < len {
            if piece_pointer == 8 {
                return Err("Invalid IPv6 address")
            }
            if input[i] == b':' {
                if compress_pointer.is_some() {
                    return Err("Invalid IPv6 address")
                }
                i += 1;
                piece_pointer += 1;
                compress_pointer = Some(piece_pointer);
                continue
            }
            let start = i;
            let end = cmp::min(len, start + 4);
            let mut value = 0u16;
            while i < end {
                match from_hex(input[i]) {
                    Some(digit) => {
                        value = value * 0x10 + digit as u16;
                        i += 1;
                    },
                    None => break
                }
            }
            if i < len {
                match input[i] {
                    b'.' => {
                        if i == start {
                            return Err("Invalid IPv6 address")
                        }
                        i = start;
                        is_ip_v4 = true;
                    },
                    b':' => {
                        i += 1;
                        if i == len {
                            return Err("Invalid IPv6 address")
                        }
                    },
                    _ => return Err("Invalid IPv6 address")
                }
            }
            if is_ip_v4 {
                break
            }
            pieces[piece_pointer] = value;
            piece_pointer += 1;
        }

        if is_ip_v4 {
            if piece_pointer > 6 {
                return Err("Invalid IPv6 address")
            }
            let mut dots_seen = 0u;
            while i < len {
                // FIXME: https://github.com/whatwg/url/commit/1c22aa119c354e0020117e02571cec53f7c01064
                let mut value = 0u16;
                while i < len {
                    let digit = match input[i] {
                        c @ b'0' .. b'9' => c - b'0',
                        _ => break
                    };
                    value = value * 10 + digit as u16;
                    if value == 0 || value > 255 {
                        return Err("Invalid IPv6 address")
                    }
                }
                if dots_seen < 3 && !(i < len && input[i] == b'.') {
                    return Err("Invalid IPv6 address")
                }
                pieces[piece_pointer] = pieces[piece_pointer] * 0x100 + value;
                if dots_seen == 0 || dots_seen == 2 {
                    piece_pointer += 1;
                }
                i += 1;
                if dots_seen == 3 && i < len {
                    return Err("Invalid IPv6 address")
                }
                dots_seen += 1;
            }
        }

        match compress_pointer {
            Some(compress_pointer) => {
                let mut swaps = piece_pointer - compress_pointer;
                piece_pointer = 7;
                while swaps > 0 {
                    pieces[piece_pointer] = pieces[compress_pointer + swaps - 1];
                    pieces[compress_pointer + swaps - 1] = 0;
                    swaps -= 1;
                    piece_pointer -= 1;
                }
            }
            _ => if piece_pointer != 8 {
                return Err("Invalid IPv6 address")
            }
        }
        Ok(Ipv6Address { pieces: pieces })
    }

    /// Serialize the IPv6 address to a string.
    pub fn serialize(&self) -> String {
        self.to_string()
    }
}


impl Show for Ipv6Address {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        let (compress_start, compress_end) = longest_zero_sequence(&self.pieces);
        let mut i = 0;
        while i < 8 {
            if i == compress_start {
                try!(formatter.write(b":"));
                if i == 0 {
                    try!(formatter.write(b":"));
                }
                if compress_end < 8 {
                    i = compress_end;
                } else {
                    break;
                }
            }
            try!(write!(formatter, "{:x}", self.pieces[i as uint]));
            if i < 7 {
                try!(formatter.write(b":"));
            }
            i += 1;
        }
        Ok(())
    }
}


fn longest_zero_sequence(pieces: &[u16, ..8]) -> (int, int) {
    let mut longest = -1;
    let mut longest_length = -1;
    let mut start = -1;
    macro_rules! finish_sequence(
        ($end: expr) => {
            if start >= 0 {
                let length = $end - start;
                if length > longest_length {
                    longest = start;
                    longest_length = length;
                }
            }
        };
    );
    for i in range(0, 8) {
        if pieces[i as uint] == 0 {
            if start < 0 {
                start = i;
            }
        } else {
            finish_sequence!(i);
            start = -1;
        }
    }
    finish_sequence!(8);
    (longest, longest + longest_length)
}


#[inline]
fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0' .. b'9' => Some(byte - b'0'),  // 0..9
        b'A' .. b'F' => Some(byte + 10 - b'A'),  // A..F
        b'a' .. b'f' => Some(byte + 10 - b'a'),  // a..f
        _ => None
    }
}


/// Represents a set of characters / bytes that should be percent-encoded.
///
/// See [encode sets specification](http://url.spec.whatwg.org/#simple-encode-set).
///
/// Different characters need to be encoded in different parts of an URL.
/// For example, a literal `?` question mark in an URL’s path would indicate
/// the start of the query string.
/// A question mark meant to be part of the path therefore needs to be percent-encoded.
/// In the query string however, a question mark does not have any special meaning
/// and does not need to be percent-encoded.
///
/// Since the implementation details of `EncodeSet` are private,
/// the set of available encode sets is not extensible beyond the ones
/// provided here.
/// If you need a different encode set,
/// please [file a bug](https://github.com/servo/rust-url/issues)
/// explaining the use case.
pub struct EncodeSet {
    map: &'static [&'static str, ..256],
}

/// This encode set is used for fragment identifier and non-relative scheme data.
pub static SIMPLE_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::SIMPLE };

/// This encode set is used in the URL parser for query strings.
pub static QUERY_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::QUERY };

/// This encode set is used for path components.
pub static DEFAULT_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::DEFAULT };

/// This encode set is used in the URL parser for usernames and passwords.
pub static USERINFO_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::USERINFO };

/// This encode set should be used when setting the password field of a parsed URL.
pub static PASSWORD_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::PASSWORD };

/// This encode set should be used when setting the username field of a parsed URL.
pub static USERNAME_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::USERNAME };

/// This encode set is used in `application/x-www-form-urlencoded` serialization.
pub static FORM_URLENCODED_ENCODE_SET: EncodeSet = EncodeSet {
    map: &encode_sets::FORM_URLENCODED,
};


/// Percent-encode the given bytes, and push the result to `output`.
///
/// The pushed strings are within the ASCII range.
#[inline]
pub fn percent_encode_to(input: &[u8], encode_set: EncodeSet, output: &mut String) {
    for &byte in input.iter() {
        output.push_str(encode_set.map[byte as uint])
    }
}


/// Percent-encode the given bytes.
///
/// The returned string is within the ASCII range.
#[inline]
pub fn percent_encode(input: &[u8], encode_set: EncodeSet) -> String {
    let mut output = String::new();
    percent_encode_to(input, encode_set, &mut output);
    output
}


/// Percent-encode the UTF-8 encoding of the given string, and push the result to `output`.
///
/// The pushed strings are within the ASCII range.
#[inline]
pub fn utf8_percent_encode_to(input: &str, encode_set: EncodeSet, output: &mut String) {
    percent_encode_to(input.as_bytes(), encode_set, output)
}


/// Percent-encode the UTF-8 encoding of the given string.
///
/// The returned string is within the ASCII range.
#[inline]
pub fn utf8_percent_encode(input: &str, encode_set: EncodeSet) -> String {
    let mut output = String::new();
    utf8_percent_encode_to(input, encode_set, &mut output);
    output
}


/// Percent-decode the given bytes, and push the result to `output`.
pub fn percent_decode_to(input: &[u8], output: &mut Vec<u8>) {
    let mut i = 0u;
    while i < input.len() {
        let c = input[i];
        if c == b'%' && i + 2 < input.len() {
            match (from_hex(input[i + 1]), from_hex(input[i + 2])) {
                (Some(h), Some(l)) => {
                    output.push(h * 0x10 + l);
                    i += 3;
                    continue
                },
                _ => (),
            }
        }

        output.push(c);
        i += 1;
    }
}


/// Percent-decode the given bytes.
#[inline]
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    percent_decode_to(input, &mut output);
    output
}


/// Percent-decode the given bytes, and decode the result as UTF-8.
///
/// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
/// will be replaced � U+FFFD, the replacement character.
#[inline]
pub fn lossy_utf8_percent_decode(input: &[u8]) -> String {
    String::from_utf8_lossy(percent_decode(input).as_slice()).into_string()
}


// FIXME: Figure out what to do on Windows
#[cfg(unix)]
fn encode_file_path(path: &Path) -> Result<Vec<String>, ()> {
    if !path.is_absolute() {
        return Err(())
    }
    Ok(path.components().map(|c| percent_encode(c, DEFAULT_ENCODE_SET)).collect())
}

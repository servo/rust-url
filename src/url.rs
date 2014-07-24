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
//! <style>.sidebar { margin-top: 53px }</style>
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
//! This is a replacement of the [`url` crate](http://doc.rust-lang.org/url/index.html)
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
//! … so that, when the renaming is done, you will only need to change this one line.
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

use encode_sets::{PASSWORD_ENCODE_SET, USERNAME_ENCODE_SET, DEFAULT_ENCODE_SET};


mod encode_sets;
mod parser;
pub mod form_urlencoded;
pub mod punycode;

#[cfg(test)]
mod tests;


#[deriving(PartialEq, Eq, Clone)]
pub struct Url {
    pub scheme: String,
    pub scheme_data: SchemeData,
    pub query: Option<String>,  // See form_urlencoded::parse_str() to get name/value pairs.
    pub fragment: Option<String>,
}

#[deriving(PartialEq, Eq, Clone)]
pub enum SchemeData {
    RelativeSchemeData(RelativeSchemeData),
    NonRelativeSchemeData(String),  // data: URLs, mailto: URLs, etc.
}

#[deriving(PartialEq, Eq, Clone)]
pub struct RelativeSchemeData {
    pub username: String,
    pub password: Option<String>,
    pub host: Host,
    pub port: String,
    pub path: Vec<String>,
}

#[deriving(PartialEq, Eq, Clone)]
pub enum Host {
    Domain(String),
    Ipv6(Ipv6Address)
}

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


pub struct UrlParser<'a> {
    base_url: Option<&'a Url>,
    query_encoding_override: Option<EncodingRef>,
    error_handler: ErrorHandler,
    scheme_type_mapper: fn(scheme: &str) -> SchemeType,
}


impl<'a> UrlParser<'a> {
    #[inline]
    pub fn new() -> UrlParser<'a> {
        UrlParser {
            base_url: None,
            query_encoding_override: None,
            error_handler: silent_handler,
            scheme_type_mapper: whatwg_scheme_type_mapper,
        }
    }

    #[inline]
    pub fn base_url<'b>(&'b mut self, value: &'a Url) -> &'b mut UrlParser<'a> {
        self.base_url = Some(value);
        self
    }

    #[inline]
    pub fn query_encoding_override<'b>(&'b mut self, value: EncodingRef) -> &'b mut UrlParser<'a> {
        self.query_encoding_override = Some(value);
        self
    }

    #[inline]
    pub fn error_handler<'b>(&'b mut self, value: ErrorHandler) -> &'b mut UrlParser<'a> {
        self.error_handler = value;
        self
    }

    #[inline]
    pub fn scheme_type_mapper<'b>(&'b mut self, value: fn(scheme: &str) -> SchemeType)
                       -> &'b mut UrlParser<'a> {
        self.scheme_type_mapper = value;
        self
    }

    #[inline]
    pub fn parse(&self, input: &str) -> ParseResult<Url> {
        parser::parse_url(input, self)
    }

    #[inline]
    fn parse_error(&self, message: &'static str) -> ParseResult<()> {
        (self.error_handler)(message)
    }

    #[inline]
    fn get_scheme_type(&self, scheme: &str) -> SchemeType {
        (self.scheme_type_mapper)(scheme)
    }
}


#[deriving(PartialEq, Eq)]
pub enum SchemeType {
    FileLikeScheme,
    RelativeScheme(&'static str),  // str is the default port, in ASCII decimal.
    NonRelativeScheme,
}

/// http://url.spec.whatwg.org/#relative-scheme
fn whatwg_scheme_type_mapper(scheme: &str) -> SchemeType {
    match scheme {
        "file" => FileLikeScheme,
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
/// The handler can choose to continue or abort parsing by returning Ok() or Err(), respectively.
/// FIXME: make this a by-ref closure when that’s supported.
pub type ErrorHandler = fn(reason: &'static str) -> ParseResult<()>;

fn silent_handler(_reason: &'static str) -> ParseResult<()> {
    Ok(())
}


impl Url {
    #[inline]
    pub fn parse(input: &str) -> ParseResult<Url> {
        UrlParser::new().parse(input)
    }

    // FIXME: Figure out what to do on Windows
    #[cfg(unix)]
    pub fn from_file_path(path: &Path) -> Result<Url, ()> {
        let path = try!(encode_file_path(path));
        Ok(Url::from_path_common(path))
    }

    // FIXME: Figure out what to do on Windows
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

    #[inline]
    pub fn to_file_path(&self) -> Result<Path, ()> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => scheme_data.to_file_path(),
            NonRelativeSchemeData(..) => Err(()),
        }
    }

    pub fn serialize(&self) -> String {
        self.to_string()
    }

    pub fn serialize_no_fragment(&self) -> String {
        UrlNoFragmentFormatter{ url: self }.to_string()
    }

    #[inline]
    pub fn non_relative_scheme_data<'a>(&'a self) -> Option<&'a str> {
        match self.scheme_data {
            RelativeSchemeData(..) => None,
            NonRelativeSchemeData(ref scheme_data) => Some(scheme_data.as_slice()),
        }
    }

    #[inline]
    pub fn relative_scheme_data<'a>(&'a self) -> Option<&'a RelativeSchemeData> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(scheme_data),
            NonRelativeSchemeData(..) => None,
        }
    }

    #[inline]
    pub fn host<'a>(&'a self) -> Option<&'a Host> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(&scheme_data.host),
            NonRelativeSchemeData(..) => None,
        }
    }

    #[inline]
    pub fn domain<'a>(&'a self) -> Option<&'a str> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => scheme_data.domain(),
            NonRelativeSchemeData(..) => None,
        }
    }

    #[inline]
    pub fn port<'a>(&'a self) -> Option<&'a str> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(scheme_data.port.as_slice()),
            NonRelativeSchemeData(..) => None,
        }
    }

    #[inline]
    pub fn path<'a>(&'a self) -> Option<&'a [String]> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(scheme_data.path.as_slice()),
            NonRelativeSchemeData(..) => None,
        }
    }

    #[inline]
    pub fn serialize_host(&self) -> Option<String> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(scheme_data.host.serialize()),
            NonRelativeSchemeData(..) => None,
        }
    }

    #[inline]
    pub fn serialize_path(&self) -> Option<String> {
        match self.scheme_data {
            RelativeSchemeData(ref scheme_data) => Some(scheme_data.serialize_path()),
            NonRelativeSchemeData(..) => None,
        }
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
    // FIXME: Figure out what to do on Windows.
    #[cfg(unix)]
    pub fn to_file_path(&self) -> Result<Path, ()> {
        // FIXME: Figure out what to do w.r.t host.
        match self.domain() {
            Some("") => {
                if self.path.is_empty() {
                    Ok(Path::new("/"))
                } else {
                    let mut bytes = Vec::new();
                    for path_part in self.path.iter() {
                        bytes.push(b'/');
                        percent_decode_to(path_part.as_bytes(), &mut bytes);
                    }
                    Ok(Path::new(bytes))
                }
            }
            _ => Err(())
        }
    }

    #[inline]
    pub fn domain<'a>(&'a self) -> Option<&'a str> {
        match self.host {
            Domain(ref domain) => Some(domain.as_slice()),
            _ => None,
        }
    }

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
                if scheme_type == FileLikeScheme {
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
        // FIXME: This is in the spec, but seems superfluous.
        match self.url.scheme_data {
            RelativeSchemeData(_) => (),
            NonRelativeSchemeData(_) => return Err("Can not set query on non-relative URL.")
        }
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
            try!(write!(formatter, "{:X}", self.pieces[i as uint]));
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


pub struct EncodeSet {
    map: &'static [&'static str, ..256],
}

pub static SIMPLE_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::SIMPLE };
pub static QUERY_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::QUERY };
pub static DEFAULT_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::DEFAULT };
pub static USERINFO_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::USERINFO };
pub static PASSWORD_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::PASSWORD };
pub static USERNAME_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::USERNAME };
pub static FORM_URLENCODED_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::FORM_URLENCODED };


#[inline]
pub fn percent_encode_to(input: &[u8], encode_set: EncodeSet, output: &mut String) {
    for &byte in input.iter() {
        output.push_str(encode_set.map[byte as uint])
    }
}


/// Percent-encode the given bytes.
///
/// The returned string. is within the ASCII range.
#[inline]
pub fn percent_encode(input: &[u8], encode_set: EncodeSet) -> String {
    let mut output = String::new();
    percent_encode_to(input, encode_set, &mut output);
    output
}


#[inline]
pub fn utf8_percent_encode_to(input: &str, encode_set: EncodeSet, output: &mut String) {
    percent_encode_to(input.as_bytes(), encode_set, output)
}


#[inline]
pub fn utf8_percent_encode(input: &str, encode_set: EncodeSet) -> String {
    let mut output = String::new();
    utf8_percent_encode_to(input, encode_set, &mut output);
    output
}


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


#[inline]
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    percent_decode_to(input, &mut output);
    output
}


// FIXME: Figure out what to do on Windows
#[cfg(unix)]
fn encode_file_path(path: &Path) -> Result<Vec<String>, ()> {
    if !path.is_absolute() {
        return Err(())
    }
    Ok(path.components().map(|c| percent_encode(c, DEFAULT_ENCODE_SET)).collect())
}

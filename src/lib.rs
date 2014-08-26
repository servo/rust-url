// Copyright 2013-2014 Simon Sapin.
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

This will automatically pull in the
[rust-encoding](https://github.com/lifthrasiir/rust-encoding) dependency.

rust-url is a replacement of the [`url` crate](http://doc.rust-lang.org/url/index.html)
currently distributed with Rust.
rust-url’s crate is also named `url`.
Cargo will automatically resolve the name conflict,
but that means that you can not also use the old `url` in the same crate.

If you’re not using Cargo, you’ll need to pass `--extern url=/path/to/liburl.rlib`
explicitly to rustc.


# URL parsing and data structures

First, URL parsing may fail for various reasons and therefore returns a `Result`.

```
use url::{Url, InvalidIpv6Address};

assert!(Url::parse("http://[:::1]") == Err(InvalidIpv6Address))
```

Let’s parse a valid URL and look at its components.

```
use url::{Url, RelativeSchemeData, NonRelativeSchemeData};

let issue_list_url = Url::parse(
    "https://github.com/rust-lang/rust/issues?labels=E-easy&state=open"
).unwrap();


assert!(issue_list_url.scheme == "https".to_string());
assert!(issue_list_url.domain() == Some("github.com"));
assert!(issue_list_url.port() == None);
assert!(issue_list_url.path() == Some(&["rust-lang".to_string(),
                                        "rust".to_string(),
                                        "issues".to_string()]));
assert!(issue_list_url.query == Some("labels=E-easy&state=open".to_string()));
assert!(issue_list_url.fragment == None);
match issue_list_url.scheme_data {
    RelativeSchemeData(..) => {},  // Expected
    NonRelativeSchemeData(..) => fail!(),
}
```

The `scheme`, `query`, and `fragment` are directly fields of the `Url` struct:
they apply to all URLs.
Every other components has accessors because they only apply to URLs said to be
“in a relative scheme”. `https` is a relative scheme, but `data` is not:

```
use url::{Url, NonRelativeSchemeData};

let data_url = Url::parse("data:text/plain,Hello#").unwrap();

assert!(data_url.scheme == "data".to_string());
assert!(data_url.scheme_data == NonRelativeSchemeData("text/plain,Hello".to_string()));
assert!(data_url.non_relative_scheme_data() == Some("text/plain,Hello"));
assert!(data_url.query == None);
assert!(data_url.fragment == Some("".to_string()));
```


# Base URL

Many contexts allow URL *references* that can be relative to a *base URL*:

```html
<link rel="stylesheet" href="../main.css">
```

Since parsed URL are absolute, giving a base is required:

```
use url::{Url, RelativeUrlWithoutBase};

assert!(Url::parse("../main.css") == Err(RelativeUrlWithoutBase))
```

`UrlParser` is a method-chaining API to provide various optional parameters
to URL parsing, including a base URL.

```
use url::{Url, UrlParser};

let this_document = Url::parse("http://servo.github.io/rust-url/url/index.html").unwrap();
let css_url = UrlParser::new().base_url(&this_document).parse("../main.css").unwrap();
assert!(css_url.serialize() == "http://servo.github.io/rust-url/main.css".to_string());
```

*/


#![feature(macro_rules, default_type_params)]

extern crate encoding;

#[cfg(test)]
extern crate serialize;

use std::fmt::{Formatter, FormatError, Show};
use std::hash;
use std::path;

use encoding::EncodingRef;

pub use host::{Host, Domain, Ipv6, Ipv6Address};
pub use parser::{
    ErrorHandler, ParseResult, ParseError,
    EmptyHost, InvalidScheme, InvalidPort, InvalidIpv6Address, InvalidDomainCharacter,
    InvalidCharacter, InvalidBackslash, InvalidPercentEncoded, InvalidAtSymbolInUser,
    ExpectedTwoSlashes, NonUrlCodePoint, RelativeUrlWithScheme, RelativeUrlWithoutBase,
    RelativeUrlWithNonRelativeBase, NonAsciiDomainsNotSupportedYet,
    CannotSetFileScheme, CannotSetJavascriptScheme, CannotSetNonRelativeScheme,
};

#[deprecated = "Moved to the `percent_encoding` module"]
pub use percent_encoding::{
    percent_decode, percent_decode_to, percent_encode, percent_encode_to,
    utf8_percent_encode, utf8_percent_encode_to, lossy_utf8_percent_decode,
    SIMPLE_ENCODE_SET, QUERY_ENCODE_SET, DEFAULT_ENCODE_SET, USERINFO_ENCODE_SET,
    PASSWORD_ENCODE_SET, USERNAME_ENCODE_SET, FORM_URLENCODED_ENCODE_SET, EncodeSet,
};

use format::{PathFormatter, UserInfoFormatter, UrlNoFragmentFormatter};

mod host;
mod parser;
mod urlutils;
pub mod percent_encoding;
pub mod form_urlencoded;
pub mod punycode;
pub mod format;

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
    /// and `lossy_percent_decode_query` methods.
    pub query: Option<String>,

    /// The fragment identifier of the URL.
    ///
    /// `None` if the `#` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, pecent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_percent_decode_fragment` method.
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
    /// See also the `lossy_percent_decode_username` method.
    pub username: String,

    /// The password of the URL.
    ///
    /// `None` if the `:` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, pecent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_percent_decode_password` method.
    pub password: Option<String>,

    /// The host of the URL, either a domain name or an IPv4 address
    pub host: Host,

    /// The port number of the URL.
    /// `None` for file-like schemes, or to indicate the default port number.
    pub port: Option<u16>,

    /// The default port number for the URL’s scheme.
    /// `None` for file-like schemes.
    pub default_port: Option<u16>,

    /// The path of the URL, as vector of pecent-encoded strings.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `serialize_path` method and,
    /// for URLs in the `file` scheme, the `to_file_path` method.
    pub path: Vec<String>,
}

/// The Authority part of a URI.
pub struct Authority<'a> {
    username: Option<&'a str>,
    password: Option<&'a str>,
    host: Option<&'a Host>,
    port: Option<u16>
}

impl<'a> Show for Authority<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), FormatError> {
        match self.username {
            Some(ref u) => {
                try!(UserInfoFormatter {
                    username: *u,
                    password: self.password
                }.fmt(fmt));
            },
            None => ()
        }
        match self.host {
            Some(ref h) => try!(h.fmt(fmt)),
            None => ()
        }
        match self.port {
            Some(ref p) => {
                try!(':'.fmt(fmt));
                try!(p.fmt(fmt));
            },
            None => ()
        }
        Ok(())
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
        fn silent_handler(_reason: ParseError) -> ParseResult<()> { Ok(()) }
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
    /// fn whatwg_scheme_type_mapper(scheme: &str) -> SchemeType {
    ///     match scheme {
    ///         "file" => FileLikeRelativeScheme,
    ///         "ftp" => RelativeScheme(21),
    ///         "gopher" => RelativeScheme(70),
    ///         "http" => RelativeScheme(80),
    ///         "https" => RelativeScheme(443),
    ///         "ws" => RelativeScheme(80),
    ///         "wss" => RelativeScheme(443),
    ///         _ => NonRelativeScheme,
    ///     }
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
    fn parse_error(&self, error: ParseError) -> ParseResult<()> {
        (self.error_handler)(error)
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
    RelativeScheme(u16),

    /// Indicate a *relative* scheme similar to the *file* scheme.
    ///
    /// For example, you might want to have distinct `git+file` and `hg+file` URL schemes.
    ///
    /// This is like `RelativeScheme` except the host can be empty, there is no port number,
    /// and path parsing has (platform-independent) quirks to support Windows filenames.
    FileLikeRelativeScheme,
}


impl SchemeType {
    pub fn default_port(&self) -> Option<u16> {
        match self {
            &RelativeScheme(default_port) => Some(default_port),
            _ => None,
        }
    }
}

/// http://url.spec.whatwg.org/#relative-scheme
pub fn whatwg_scheme_type_mapper(scheme: &str) -> SchemeType {
    match scheme {
        "file" => FileLikeRelativeScheme,
        "ftp" => RelativeScheme(21),
        "gopher" => RelativeScheme(70),
        "http" => RelativeScheme(80),
        "https" => RelativeScheme(443),
        "ws" => RelativeScheme(80),
        "wss" => RelativeScheme(443),
        _ => NonRelativeScheme,
    }
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
    /// This returns `Err` if the given path is not absolute
    /// or, with a Windows path, if the prefix is not a disk prefix (e.g. `C:`).
    pub fn from_file_path<T: ToUrlPath>(path: &T) -> Result<Url, ()> {
        let path = try!(path.to_url_path());
        Ok(Url::from_path_common(path))
    }

    /// Convert a directory name as `std::path::Path` into an URL in the `file` scheme.
    ///
    /// This returns `Err` if the given path is not absolute
    /// or, with a Windows path, if the prefix is not a disk prefix (e.g. `C:`).
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
    pub fn from_directory_path<T: ToUrlPath>(path: &T) -> Result<Url, ()> {
        let mut path = try!(path.to_url_path());
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
                port: None,
                default_port: None,
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
    /// The return type (when `Ok()`) is generic and can be either `std::path::posix::Path`
    /// or `std::path::windows::Path`.
    /// (Use `std::path::Path` to pick one of them depending on the local system.)
    /// If the compiler can not infer the desired type from context, you may have to specifiy it:
    ///
    /// ```ignore
    /// let path = url.to_file_path::<std::path::posix::Path>();
    /// ```
    ///
    /// Returns `Err` if the host is neither empty nor `"localhost"`,
    /// or if `Path::new_opt()` returns `None`.
    /// (That is, if the percent-decoded path contains a NUL byte or,
    /// for a Windows path, is not UTF-8.)
    #[inline]
    pub fn to_file_path<T: FromUrlPath>(&self) -> Result<T, ()> {
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

    /// If the URL is *non-relative*, return a mutable reference to the string scheme data.
    #[inline]
    pub fn non_relative_scheme_data_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        match self.scheme_data {
            RelativeSchemeData(..) => None,
            NonRelativeSchemeData(ref mut scheme_data) => Some(scheme_data),
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

    /// If the URL is in a *relative scheme*,
    /// return a mutable reference to the structured scheme data.
    #[inline]
    pub fn relative_scheme_data_mut<'a>(&'a mut self) -> Option<&'a mut RelativeSchemeData> {
        match self.scheme_data {
            RelativeSchemeData(ref mut scheme_data) => Some(scheme_data),
            NonRelativeSchemeData(..) => None,
        }
    }

    /// If the URL is in a *relative scheme*, return its Authority.
    #[inline]
    pub fn authority<'a>(&'a self) -> Option<Authority<'a>> {
        self.relative_scheme_data().and(Some(Authority {
            username: self.username(),
            password: self.password(),
            host: self.host(),
            port: self.port()
        }))
    }

    /// If the URL is in a *relative scheme*, return its username.
    #[inline]
    pub fn username<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.username.as_slice())
    }

    /// If the URL is in a *relative scheme*, return a mutable reference to its username.
    #[inline]
    pub fn username_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        self.relative_scheme_data_mut().map(|scheme_data| &mut scheme_data.username)
    }

    /// Percent-decode the URL’s username, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_percent_decode_username(&self) -> Option<String> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.lossy_percent_decode_username())
    }

    /// If the URL is in a *relative scheme*, return its password, if any.
    #[inline]
    pub fn password<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().and_then(|scheme_data|
            scheme_data.password.as_ref().map(|password| password.as_slice()))
    }

    /// If the URL is in a *relative scheme*, return a mutable reference to its password, if any.
    #[inline]
    pub fn password_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        self.relative_scheme_data_mut().and_then(|scheme_data| scheme_data.password.as_mut())
    }

    /// Percent-decode the URL’s password, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_percent_decode_password(&self) -> Option<String> {
        self.relative_scheme_data().and_then(|scheme_data|
            scheme_data.lossy_percent_decode_password())
    }

    /// Serialize the URL's username and password, if any.
    ///
    /// Format: "<username>:<password>@"
    #[inline]
    pub fn serialize_userinfo<'a>(&'a mut self) -> Option<String> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.serialize_userinfo())
    }

    /// If the URL is in a *relative scheme*, return its structured host.
    #[inline]
    pub fn host<'a>(&'a self) -> Option<&'a Host> {
        self.relative_scheme_data().map(|scheme_data| &scheme_data.host)
    }

    /// If the URL is in a *relative scheme*, return a mutable reference to its structured host.
    #[inline]
    pub fn host_mut<'a>(&'a mut self) -> Option<&'a mut Host> {
        self.relative_scheme_data_mut().map(|scheme_data| &mut scheme_data.host)
    }

    /// If the URL is in a *relative scheme* and its host is a domain,
    /// return the domain as a string.
    #[inline]
    pub fn domain<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().and_then(|scheme_data| scheme_data.domain())
    }

    /// If the URL is in a *relative scheme* and its host is a domain,
    /// return a mutable reference to the domain string.
    #[inline]
    pub fn domain_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        self.relative_scheme_data_mut().and_then(|scheme_data| scheme_data.domain_mut())
    }

    /// If the URL is in a *relative scheme*, serialize its host as a string.
    ///
    /// A domain a returned as-is, an IPv6 address between [] square brackets.
    #[inline]
    pub fn serialize_host(&self) -> Option<String> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.host.serialize())
    }

    /// If the URL is in a *relative scheme* and has a port number, return it.
    #[inline]
    pub fn port<'a>(&'a self) -> Option<u16> {
        self.relative_scheme_data().and_then(|scheme_data| scheme_data.port)
    }

    /// If the URL is in a *relative scheme*, return a mutable reference to its port.
    #[inline]
    pub fn port_mut<'a>(&'a mut self) -> Option<&'a mut Option<u16>> {
        self.relative_scheme_data_mut().map(|scheme_data| &mut scheme_data.port)
    }

    /// If the URL is in a *relative scheme* that is not a file-like,
    /// return its port number, even if it is the default.
    #[inline]
    pub fn port_or_default(&self) -> Option<u16> {
        self.relative_scheme_data().and_then(|scheme_data| scheme_data.port_or_default())
    }

    /// If the URL is in a *relative scheme*, return its path components.
    #[inline]
    pub fn path<'a>(&'a self) -> Option<&'a [String]> {
        self.relative_scheme_data().map(|scheme_data| scheme_data.path.as_slice())
    }

    /// If the URL is in a *relative scheme*, return a mutable reference to its path components.
    #[inline]
    pub fn path_mut<'a>(&'a mut self) -> Option<&'a mut Vec<String>> {
        self.relative_scheme_data_mut().map(|scheme_data| &mut scheme_data.path)
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
    pub fn lossy_percent_decode_query(&self) -> Option<String> {
        self.query.as_ref().map(|value| lossy_utf8_percent_decode(value.as_bytes()))
    }

    /// Percent-decode the URL’s fragment identifier, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_percent_decode_fragment(&self) -> Option<String> {
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
    pub fn lossy_percent_decode_username(&self) -> String {
        lossy_utf8_percent_decode(self.username.as_bytes())
    }

    /// Percent-decode the URL’s password, if any.
    ///
    /// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
    /// will be replaced � U+FFFD, the replacement character.
    #[inline]
    pub fn lossy_percent_decode_password(&self) -> Option<String> {
        self.password.as_ref().map(|value| lossy_utf8_percent_decode(value.as_bytes()))
    }

    /// Assuming the URL is in the `file` scheme or similar,
    /// convert its path to an absolute `std::path::Path`.
    ///
    /// **Note:** This does not actually check the URL’s `scheme`,
    /// and may give nonsensical results for other schemes.
    /// It is the user’s responsibility to check the URL’s scheme before calling this.
    ///
    /// The return type (when `Ok()`) is generic and can be either `std::path::posix::Path`
    /// or `std::path::windows::Path`.
    /// (Use `std::path::Path` to pick one of them depending on the local system.)
    /// If the compiler can not infer the desired type from context, you may have to specifiy it:
    ///
    /// ```ignore
    /// let path = url.to_file_path::<std::path::posix::Path>();
    /// ```
    ///
    /// Returns `Err` if the host is neither empty nor `"localhost"`,
    /// or if `Path::new_opt()` returns `None`.
    /// (That is, if the percent-decoded path contains a NUL byte or,
    /// for a Windows path, is not UTF-8.)
    #[inline]
    pub fn to_file_path<T: FromUrlPath>(&self) -> Result<T, ()> {
        // FIXME: Figure out what to do w.r.t host.
        match self.domain() {
            Some("") | Some("localhost") => FromUrlPath::from_url_path(self.path.as_slice()),
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

    /// If the host is a domain, return a mutable reference to the domain string.
    #[inline]
    pub fn domain_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        match self.host {
            Domain(ref mut domain) => Some(domain),
            _ => None,
        }
    }

    /// Return the port number of the URL, even if it is the default.
    /// Return `None` for file-like URLs.
    #[inline]
    pub fn port_or_default(&self) -> Option<u16> {
        self.port.or(self.default_port)
    }

    /// Serialize the path as a string.
    ///
    /// The returned string starts with a "/" slash, and components are separated by slashes.
    /// A trailing slash represents an empty last component.
    pub fn serialize_path(&self) -> String {
        PathFormatter {
            path: self.path.as_slice()
        }.to_string()
    }

    /// Serialize the userinfo as a string.
    ///
    /// Format: "<username>:<password>@".
    pub fn serialize_userinfo(&self) -> String {
        UserInfoFormatter {
            username: self.username.as_slice(),
            password: self.password.as_ref().map(|s| s.as_slice())
        }.to_string()
    }
}


impl Show for RelativeSchemeData {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), FormatError> {
        // Write the scheme-trailing double slashes.
        try!(formatter.write(b"//"));

        // Write the user info.
        try!(UserInfoFormatter {
            username: self.username.as_slice(),
            password: self.password.as_ref().map(|s| s.as_slice())
        }.fmt(formatter));

        // Write the host.
        try!(self.host.fmt(formatter));

        // Write the port.
        match self.port {
            Some(port) => {
                try!(write!(formatter, ":{}", port));
            },
            None => {}
        }

        // Write the path.
        PathFormatter {
            path: self.path.as_slice()
        }.fmt(formatter)
    }
}


trait ToUrlPath {
    fn to_url_path(&self) -> Result<Vec<String>, ()>;
}


impl ToUrlPath for path::posix::Path {
    fn to_url_path(&self) -> Result<Vec<String>, ()> {
        if !self.is_absolute() {
            return Err(())
        }
        Ok(self.components().map(|c| percent_encode(c, DEFAULT_ENCODE_SET)).collect())
    }
}


impl ToUrlPath for path::windows::Path {
    fn to_url_path(&self) -> Result<Vec<String>, ()> {
        if !self.is_absolute() {
            return Err(())
        }
        if path::windows::prefix(self) != Some(path::windows::DiskPrefix) {
            // FIXME: do something with UNC and other prefixes?
            return Err(())
        }
        // Start with the prefix, e.g. "C:"
        let mut path = vec![self.as_str().unwrap().slice_to(2).to_string()];
        // self.components() does not include the prefix
        for component in self.components() {
            path.push(percent_encode(component, DEFAULT_ENCODE_SET));
        }
        Ok(path)
    }
}


trait FromUrlPath {
    fn from_url_path(path: &[String]) -> Result<Self, ()>;
}


impl FromUrlPath for path::posix::Path {
    fn from_url_path(path: &[String]) -> Result<path::posix::Path, ()> {
        if path.is_empty() {
            return Ok(path::posix::Path::new("/"))
        }
        let mut bytes = Vec::new();
        for path_part in path.iter() {
            bytes.push(b'/');
            percent_decode_to(path_part.as_bytes(), &mut bytes);
        }
        match path::posix::Path::new_opt(bytes) {
            None => Err(()),  // Path contains a NUL byte
            Some(path) => {
                debug_assert!(path.is_absolute(),
                              "to_file_path() failed to produce an absolute Path")
                Ok(path)
            }
        }
    }
}


impl FromUrlPath for path::windows::Path {
    fn from_url_path(path: &[String]) -> Result<path::windows::Path, ()> {
        if path.is_empty() {
            return Err(())
        }
        let prefix = path[0].as_slice();
        if prefix.len() != 2 || !parser::starts_with_ascii_alpha(prefix)
                || prefix.char_at(1) != ':' {
            return Err(())
        }
        let mut bytes = prefix.as_bytes().to_vec();
        for path_part in path.slice_from(1).iter() {
            bytes.push(b'\\');
            percent_decode_to(path_part.as_bytes(), &mut bytes);
        }
        match path::windows::Path::new_opt(bytes) {
            None => Err(()),  // Path contains a NUL byte or invalid UTF-8
            Some(path) => {
                debug_assert!(path.is_absolute(),
                              "to_file_path() failed to produce an absolute Path")
                debug_assert!(path::windows::prefix(&path) == Some(path::windows::DiskPrefix),
                              "to_file_path() failed to produce a Path with a disk prefix")
                Ok(path)
            }
        }
    }
}

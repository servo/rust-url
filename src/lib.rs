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
use url::{Url, SchemeData};

let issue_list_url = Url::parse(
    "https://github.com/rust-lang/rust/issues?labels=E-easy&state=open"
).unwrap();


assert!(issue_list_url.scheme == "https".to_string());
assert!(issue_list_url.domain() == Some("github.com"));
assert!(issue_list_url.port() == None);
assert!(issue_list_url.path() == Some(&["rust-lang".to_string(),
                                        "rust".to_string(),
                                        "issues".to_string()][..]));
assert!(issue_list_url.query == Some("labels=E-easy&state=open".to_string()));
assert!(issue_list_url.fragment == None);
match issue_list_url.scheme_data {
    SchemeData::Relative(..) => {},  // Expected
    SchemeData::NonRelative(..) => panic!(),
}
```

The `scheme`, `query`, and `fragment` are directly fields of the `Url` struct:
they apply to all URLs.
Every other components has accessors because they only apply to URLs said to be
“in a relative scheme”. `https` is a relative scheme, but `data` is not:

```
use url::{Url, SchemeData};

let data_url = Url::parse("data:text/plain,Hello#").unwrap();

assert!(data_url.scheme == "data".to_string());
assert!(data_url.scheme_data == SchemeData::NonRelative("text/plain,Hello".to_string()));
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
use url::{Url, ParseError};

assert!(Url::parse("../main.css") == Err(ParseError::RelativeUrlWithoutBase))
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

extern crate rustc_serialize;

#[macro_use]
extern crate matches;

#[cfg(feature="serde_serialization")]
extern crate serde;

use std::fmt::{self, Formatter};
use std::str;
use std::path::{Path, PathBuf};

#[cfg(feature="serde_serialization")]
use std::str::FromStr;

pub use host::{Host, Ipv6Address};
pub use parser::{ErrorHandler, ParseResult, ParseError};

use percent_encoding::{percent_encode, lossy_utf8_percent_decode, DEFAULT_ENCODE_SET};

use format::{PathFormatter, UserInfoFormatter, UrlNoFragmentFormatter};
use encoding::EncodingOverride;

mod encoding;
mod host;
mod parser;
pub mod urlutils;
pub mod percent_encoding;
pub mod form_urlencoded;
pub mod punycode;
pub mod format;

#[cfg(test)]
mod tests;


/// The parsed representation of an absolute URL.
#[derive(PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord)]
pub struct Url {
    /// The scheme (a.k.a. protocol) of the URL, in ASCII lower case.
    pub scheme: String,

    /// The components of the URL whose representation depends on where the scheme is *relative*.
    pub scheme_data: SchemeData,

    /// The query string of the URL.
    ///
    /// `None` if the `?` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, percent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `query_pairs`, `set_query_from_pairs`,
    /// and `lossy_percent_decode_query` methods.
    pub query: Option<String>,

    /// The fragment identifier of the URL.
    ///
    /// `None` if the `#` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, percent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_percent_decode_fragment` method.
    pub fragment: Option<String>,
}

/// The components of the URL whose representation depends on where the scheme is *relative*.
#[derive(PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord)]
pub enum SchemeData {
    /// Components for URLs in a *relative* scheme such as HTTP.
    Relative(RelativeSchemeData),

    /// No further structure is assumed for *non-relative* schemes such as `data` and `mailto`.
    ///
    /// This is a single percent-encoded string, whose interpretation depends on the scheme.
    ///
    /// Percent encoded strings are within the ASCII range.
    NonRelative(String),
}

/// Components for URLs in a *relative* scheme such as HTTP.
#[derive(PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord)]
pub struct RelativeSchemeData {
    /// The username of the URL, as a possibly empty, percent-encoded string.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `lossy_percent_decode_username` method.
    pub username: String,

    /// The password of the URL.
    ///
    /// `None` if the `:` delimiter character was not part of the parsed input,
    /// otherwise a possibly empty, percent-encoded string.
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

    /// The path of the URL, as vector of percent-encoded strings.
    ///
    /// Percent encoded strings are within the ASCII range.
    ///
    /// See also the `serialize_path` method and,
    /// for URLs in the `file` scheme, the `to_file_path` method.
    pub path: Vec<String>,
}


impl str::FromStr for Url {
    type Err = ParseError;

    fn from_str(url: &str) -> ParseResult<Url> {
        Url::parse(url)
    }
}

/// A set of optional parameters for URL parsing.
pub struct UrlParser<'a> {
    base_url: Option<&'a Url>,
    query_encoding_override: EncodingOverride,
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
            query_encoding_override: EncodingOverride::utf8(),
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
    ///
    /// This method is only available if the `query_encoding` Cargo feature is enabled.
    #[cfg(feature = "query_encoding")]
    #[inline]
    pub fn query_encoding_override<'b>(&'b mut self, value: encoding::EncodingRef)
                                       -> &'b mut UrlParser<'a> {
        self.query_encoding_override = EncodingOverride::from_encoding(value);
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
    ///         "file" => SchemeType::FileLike,
    ///         "ftp" => SchemeType::Relative(21),
    ///         "gopher" => SchemeType::Relative(70),
    ///         "http" => SchemeType::Relative(80),
    ///         "https" => SchemeType::Relative(443),
    ///         "ws" => SchemeType::Relative(80),
    ///         "wss" => SchemeType::Relative(443),
    ///         _ => NonRelative,
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

    /// Parse `input` as a “standalone” URL path,
    /// with an optional query string and fragment identifier.
    ///
    /// This is typically found in the start line of an HTTP header.
    ///
    /// Note that while the start line has no fragment identifier in the HTTP RFC,
    /// servers typically parse it and ignore it
    /// (rather than having it be part of the path or query string.)
    ///
    /// On success, return `(path, query_string, fragment_identifier)`
    #[inline]
    pub fn parse_path(&self, input: &str)
                      -> ParseResult<(Vec<String>, Option<String>, Option<String>)> {
        parser::parse_standalone_path(input, self)
    }
}


/// Parse `input` as a “standalone” URL path,
/// with an optional query string and fragment identifier.
///
/// This is typically found in the start line of an HTTP header.
///
/// Note that while the start line has no fragment identifier in the HTTP RFC,
/// servers typically parse it and ignore it
/// (rather than having it be part of the path or query string.)
///
/// On success, return `(path, query_string, fragment_identifier)`
///
/// ```rust
/// let (path, query, fragment) = url::parse_path("/foo/bar/../baz?q=42").unwrap();
/// assert_eq!(path, vec!["foo".to_string(), "baz".to_string()]);
/// assert_eq!(query, Some("q=42".to_string()));
/// assert_eq!(fragment, None);
/// ```
#[inline]
pub fn parse_path(input: &str)
                  -> ParseResult<(Vec<String>, Option<String>, Option<String>)> {
    UrlParser::new().parse_path(input)
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
#[derive(PartialEq, Eq, Copy, Debug, Clone, Hash, PartialOrd, Ord)]
pub enum SchemeType {
    /// Indicate that the scheme is *non-relative*.
    ///
    /// The *scheme data* of the URL
    /// (everything other than the scheme, query string, and fragment identifier)
    /// is parsed as a single percent-encoded string of which no structure is assumed.
    /// That string may need to be parsed further, per a scheme-specific format.
    NonRelative,

    /// Indicate that the scheme is *relative*, and what the default port number is.
    ///
    /// The *scheme data* is structured as
    /// *username*, *password*, *host*, *port number*, and *path*.
    /// Relative URL references are supported, if a base URL was given.
    /// The string value indicates the default port number as a string of ASCII digits,
    /// or the empty string to indicate no default port number.
    Relative(u16),

    /// Indicate a *relative* scheme similar to the *file* scheme.
    ///
    /// For example, you might want to have distinct `git+file` and `hg+file` URL schemes.
    ///
    /// This is like `Relative` except the host can be empty, there is no port number,
    /// and path parsing has (platform-independent) quirks to support Windows filenames.
    FileLike,
}

impl SchemeType {
    pub fn default_port(&self) -> Option<u16> {
        match self {
            &SchemeType::Relative(default_port) => Some(default_port),
            _ => None,
        }
    }
    pub fn same_as(&self, other: SchemeType) -> bool {
        match (self, other) {
            (&SchemeType::NonRelative, SchemeType::NonRelative) => true,
            (&SchemeType::Relative(_), SchemeType::Relative(_)) => true,
            (&SchemeType::FileLike,    SchemeType::FileLike) => true,
            _ => false
        }
    }
}

/// http://url.spec.whatwg.org/#relative-scheme
pub fn whatwg_scheme_type_mapper(scheme: &str) -> SchemeType {
    match scheme {
        "file" => SchemeType::FileLike,
        "ftp" => SchemeType::Relative(21),
        "gopher" => SchemeType::Relative(70),
        "http" => SchemeType::Relative(80),
        "https" => SchemeType::Relative(443),
        "ws" => SchemeType::Relative(80),
        "wss" => SchemeType::Relative(443),
        _ => SchemeType::NonRelative,
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
    pub fn from_file_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let path = try!(path_to_file_url_path(path.as_ref()));
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
    pub fn from_directory_path<P: AsRef<Path>>(path: P) -> Result<Url, ()> {
        let mut path = try!(path_to_file_url_path(path.as_ref()));
        // Add an empty path component (i.e. a trailing slash in serialization)
        // so that the entire path is used as a base URL.
        path.push("".to_string());
        Ok(Url::from_path_common(path))
    }

    fn from_path_common(path: Vec<String>) -> Url {
        Url {
            scheme: "file".to_string(),
            scheme_data: SchemeData::Relative(RelativeSchemeData {
                username: "".to_string(),
                password: None,
                port: None,
                default_port: None,
                host: Host::Domain("".to_string()),
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
    /// If the compiler can not infer the desired type from context, you may have to specify it:
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
    pub fn to_file_path(&self) -> Result<PathBuf, ()> {
        match self.scheme_data {
            SchemeData::Relative(ref scheme_data) => scheme_data.to_file_path(),
            SchemeData::NonRelative(..) => Err(()),
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
            SchemeData::Relative(..) => None,
            SchemeData::NonRelative(ref scheme_data) => Some(scheme_data),
        }
    }

    /// If the URL is *non-relative*, return a mutable reference to the string scheme data.
    #[inline]
    pub fn non_relative_scheme_data_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        match self.scheme_data {
            SchemeData::Relative(..) => None,
            SchemeData::NonRelative(ref mut scheme_data) => Some(scheme_data),
        }
    }

    /// If the URL is in a *relative scheme*, return the structured scheme data.
    #[inline]
    pub fn relative_scheme_data<'a>(&'a self) -> Option<&'a RelativeSchemeData> {
        match self.scheme_data {
            SchemeData::Relative(ref scheme_data) => Some(scheme_data),
            SchemeData::NonRelative(..) => None,
        }
    }

    /// If the URL is in a *relative scheme*,
    /// return a mutable reference to the structured scheme data.
    #[inline]
    pub fn relative_scheme_data_mut<'a>(&'a mut self) -> Option<&'a mut RelativeSchemeData> {
        match self.scheme_data {
            SchemeData::Relative(ref mut scheme_data) => Some(scheme_data),
            SchemeData::NonRelative(..) => None,
        }
    }

    /// If the URL is in a *relative scheme*, return its username.
    #[inline]
    pub fn username<'a>(&'a self) -> Option<&'a str> {
        self.relative_scheme_data().map(|scheme_data| &*scheme_data.username)
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
            scheme_data.password.as_ref().map(|password| password as &str))
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
        self.relative_scheme_data().map(|scheme_data| &*scheme_data.path)
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
        self.query.as_ref().map(|query| form_urlencoded::parse(query.as_bytes()))
    }

    /// Serialize an iterator of (key, value) pairs as `application/x-www-form-urlencoded`
    /// and set it as the URL’s query string.
    #[inline]
    pub fn set_query_from_pairs<'a, I: Iterator<Item = (&'a str, &'a str)>>(&mut self, pairs: I) {
        self.query = Some(form_urlencoded::serialize(pairs));
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


impl rustc_serialize::Encodable for Url {
    fn encode<S: rustc_serialize::Encoder>(&self, encoder: &mut S) -> Result<(), S::Error> {
        encoder.emit_str(&self.to_string())
    }
}


impl rustc_serialize::Decodable for Url {
    fn decode<D: rustc_serialize::Decoder>(decoder: &mut D) -> Result<Url, D::Error> {
        Url::parse(&*try!(decoder.read_str())).map_err(|error| {
            decoder.error(&format!("URL parsing error: {}", error))
        })
    }
}

/// Serializes this URL into a `serde` stream.
///
/// This implementation is only available if the `serde_serialization` Cargo feature is enabled.
#[cfg(feature="serde_serialization")]
impl serde::Serialize for Url {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: serde::Serializer {
        format!("{}", self).serialize(serializer)
    }
}

/// Deserializes this URL from a `serde` stream.
///
/// This implementation is only available if the `serde_serialization` Cargo feature is enabled.
#[cfg(feature="serde_serialization")]
impl serde::Deserialize for Url {
    fn deserialize<D>(deserializer: &mut D) -> Result<Url, D::Error> where D: serde::Deserializer {
        let string_representation: String = try!(serde::Deserialize::deserialize(deserializer));
        Ok(FromStr::from_str(&string_representation[..]).unwrap())
    }
}

impl fmt::Display for Url {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        try!(UrlNoFragmentFormatter{ url: self }.fmt(formatter));
        if let Some(ref fragment) = self.fragment {
            try!(formatter.write_str("#"));
            try!(formatter.write_str(fragment));
        }
        Ok(())
    }
}


impl fmt::Display for SchemeData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match *self {
            SchemeData::Relative(ref scheme_data) => scheme_data.fmt(formatter),
            SchemeData::NonRelative(ref scheme_data) => scheme_data.fmt(formatter),
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
    pub fn to_file_path(&self) -> Result<PathBuf, ()> {
        // FIXME: Figure out what to do w.r.t host.
        if !matches!(self.domain(), Some("") | Some("localhost")) {
            return Err(())
        }
        file_url_path_to_pathbuf(&self.path)
    }

    /// If the host is a domain, return the domain as a string.
    #[inline]
    pub fn domain<'a>(&'a self) -> Option<&'a str> {
        match self.host {
            Host::Domain(ref domain) => Some(domain),
            _ => None,
        }
    }

    /// If the host is a domain, return a mutable reference to the domain string.
    #[inline]
    pub fn domain_mut<'a>(&'a mut self) -> Option<&'a mut String> {
        match self.host {
            Host::Domain(ref mut domain) => Some(domain),
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
            path: &self.path
        }.to_string()
    }

    /// Serialize the userinfo as a string.
    ///
    /// Format: "<username>:<password>@".
    pub fn serialize_userinfo(&self) -> String {
        UserInfoFormatter {
            username: &self.username,
            password: self.password.as_ref().map(|s| s as &str)
        }.to_string()
    }
}


impl fmt::Display for RelativeSchemeData {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        // Write the scheme-trailing double slashes.
        try!(formatter.write_str("//"));

        // Write the user info.
        try!(UserInfoFormatter {
            username: &self.username,
            password: self.password.as_ref().map(|s| s as &str)
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
            path: &self.path
        }.fmt(formatter)
    }
}


#[cfg(unix)]
fn path_to_file_url_path(path: &Path) -> Result<Vec<String>, ()> {
    use std::os::unix::prelude::OsStrExt;
    if !path.is_absolute() {
        return Err(())
    }
    // skip the root component
    Ok(path.components().skip(1).map(|c| {
        percent_encode(c.as_os_str().as_bytes(), DEFAULT_ENCODE_SET)
    }).collect())
}

#[cfg(windows)]
fn path_to_file_url_path(path: &Path) -> Result<Vec<String>, ()> {
    path_to_file_url_path_windows(path)
}

// Build this unconditionally to alleviate https://github.com/servo/rust-url/issues/102
#[cfg_attr(not(windows), allow(dead_code))]
fn path_to_file_url_path_windows(path: &Path) -> Result<Vec<String>, ()> {
    use std::path::{Prefix, Component};
    if !path.is_absolute() {
        return Err(())
    }
    let mut components = path.components();
    let disk = match components.next() {
        Some(Component::Prefix(ref p)) => match p.kind() {
            Prefix::Disk(byte) => byte,
            _ => return Err(()),
        },

        // FIXME: do something with UNC and other prefixes?
        _ => return Err(())
    };

    // Start with the prefix, e.g. "C:"
    let mut path = vec![format!("{}:", disk as char)];

    for component in components {
        if component == Component::RootDir { continue }
        // FIXME: somehow work with non-unicode?
        let part = match component.as_os_str().to_str() {
            Some(s) => s,
            None => return Err(()),
        };
        path.push(percent_encode(part.as_bytes(), DEFAULT_ENCODE_SET));
    }
    Ok(path)
}

#[cfg(unix)]
fn file_url_path_to_pathbuf(path: &[String]) -> Result<PathBuf, ()> {
    use std::ffi::OsStr;
    use std::os::unix::prelude::OsStrExt;
    use std::path::PathBuf;

    use percent_encoding::percent_decode_to;

    if path.is_empty() {
        return Ok(PathBuf::from("/"))
    }
    let mut bytes = Vec::new();
    for path_part in path {
        bytes.push(b'/');
        percent_decode_to(path_part.as_bytes(), &mut bytes);
    }
    let os_str = OsStr::from_bytes(&bytes);
    let path = PathBuf::from(os_str);
    debug_assert!(path.is_absolute(),
                  "to_file_path() failed to produce an absolute Path");
    Ok(path)
}

#[cfg(windows)]
fn file_url_path_to_pathbuf(path: &[String]) -> Result<PathBuf, ()> {
    file_url_path_to_pathbuf_windows(path)
}

// Build this unconditionally to alleviate https://github.com/servo/rust-url/issues/102
#[cfg_attr(not(windows), allow(dead_code))]
fn file_url_path_to_pathbuf_windows(path: &[String]) -> Result<PathBuf, ()> {
    use percent_encoding::percent_decode;

    if path.is_empty() {
        return Err(())
    }
    let prefix = &*path[0];
    if prefix.len() != 2 || !parser::starts_with_ascii_alpha(prefix)
            || prefix.as_bytes()[1] != b':' {
        return Err(())
    }
    let mut string = prefix.to_string();
    for path_part in &path[1..] {
        string.push('\\');

        // Currently non-unicode windows paths cannot be represented
        match String::from_utf8(percent_decode(path_part.as_bytes())) {
            Ok(s) => string.push_str(&s),
            Err(..) => return Err(()),
        }
    }
    let path = PathBuf::from(string);
    debug_assert!(path.is_absolute(),
                  "to_file_path() failed to produce an absolute Path");
    Ok(path)
}

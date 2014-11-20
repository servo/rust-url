// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
pub use ParseError::{
    EmptyHost,InvalidScheme,InvalidPort,InvalidIpv6Address,InvalidDomainCharacter,
    InvalidCharacter,InvalidBackslash,InvalidPercentEncoded,InvalidAtSymbolInUser,
    ExpectedTwoSlashes,ExpectedInitialSlash,NonUrlCodePoint,RelativeUrlWithScheme,
    RelativeUrlWithoutBase,RelativeUrlWithNonRelativeBase,
    NonAsciiDomainsNotSupportedYet,CannotSetFileScheme,
    CannotSetJavascriptScheme,CannotSetNonRelativeScheme
};

pub use self::Context::{SetterContext,UrlParserContext};
use std::ascii::AsciiExt;
use std::fmt::{mod, Formatter, Show};
use std::str::CharRange;

use encoding;

use super::{
    UrlParser, Url, RelativeSchemeData, NonRelativeSchemeData, Host, Domain,
    SchemeType, FileLikeRelativeScheme, RelativeScheme, NonRelativeScheme,
    UrlRelativeSchemeData,
};
use percent_encoding::{
    utf8_percent_encode_to, percent_encode,
    SIMPLE_ENCODE_SET, DEFAULT_ENCODE_SET, USERINFO_ENCODE_SET, QUERY_ENCODE_SET
};


macro_rules! is_match(
    ($value:expr, $($pattern:pat)|+) => (
        match $value { $($pattern)|+ => true, _ => false }
    );
)


pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing.
#[deriving(PartialEq, Eq, Clone)]
pub enum ParseError {
    EmptyHost,
    InvalidScheme,
    InvalidPort,
    InvalidIpv6Address,
    InvalidDomainCharacter,
    InvalidCharacter,
    InvalidBackslash,
    InvalidPercentEncoded,
    InvalidAtSymbolInUser,
    ExpectedTwoSlashes,
    ExpectedInitialSlash,
    NonUrlCodePoint,
    RelativeUrlWithScheme,
    RelativeUrlWithoutBase,
    RelativeUrlWithNonRelativeBase,
    NonAsciiDomainsNotSupportedYet,
    CannotSetFileScheme(&'static str),
    CannotSetJavascriptScheme(&'static str),
    CannotSetNonRelativeScheme(&'static str)
}

impl Show for ParseError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            EmptyHost => "empty host",
            InvalidScheme => "invalid scheme",
            InvalidPort => "invalid port number",
            InvalidIpv6Address => "invalid IPv6 address",
            InvalidDomainCharacter => "invalid domain character",
            InvalidCharacter => "invalid character",
            InvalidBackslash => "invalid backslash",
            InvalidPercentEncoded => "invalid percent-encoded sequence",
            InvalidAtSymbolInUser => "invalid @-symbol in user",
            ExpectedTwoSlashes => "expected two slashes (//)",
            ExpectedInitialSlash => "expected the input to start with a slash",
            NonUrlCodePoint => "non URL code point",
            RelativeUrlWithScheme => "relative URL with scheme",
            RelativeUrlWithoutBase => "relative URL without a base",
            RelativeUrlWithNonRelativeBase => "relative URL with a non-relative base",
            NonAsciiDomainsNotSupportedYet => "non-ASCII domains are not supported yet",
            CannotSetFileScheme(ref part) =>
                return write!(fmt, "cannot set {} on file: URLs", part),
            CannotSetJavascriptScheme(ref part) =>
                return write!(fmt, "cannot set {} on javascript: URLs", part),
            CannotSetNonRelativeScheme(ref part) =>
                return write!(fmt, "cannot set {} on non-relative URLs", part),
        }.fmt(fmt)
    }
}

/// This is called on non-fatal parse errors.
///
/// The handler can choose to continue or abort parsing by returning Ok() or Err(), respectively.
/// See the `UrlParser::error_handler` method.
///
/// FIXME: make this a by-ref closure when that’s supported.
pub type ErrorHandler = fn(reason: ParseError) -> ParseResult<()>;


#[deriving(PartialEq, Eq)]
pub enum Context {
    UrlParserContext,
    SetterContext,
}


pub fn parse_url(input: &str, parser: &UrlParser) -> ParseResult<Url> {
    let input = input.trim_chars([' ', '\t', '\n', '\r', '\x0C'].as_slice());
    let (scheme, remaining) = match parse_scheme(input, UrlParserContext) {
        Some((scheme, remaining)) => (scheme, remaining),
        // No-scheme state
        None => return match parser.base_url {
            Some(&Url { ref scheme, scheme_data: RelativeSchemeData(ref base),
                        ref query, .. }) => {
                let scheme_type = parser.get_scheme_type(scheme.as_slice());
                parse_relative_url(input, scheme.clone(), scheme_type, base, query, parser)
            },
            Some(_) => Err(RelativeUrlWithNonRelativeBase),
            None => Err(RelativeUrlWithoutBase),
        },
    };
    let scheme_type = parser.get_scheme_type(scheme.as_slice());
    match scheme_type {
        FileLikeRelativeScheme => {
            // Relative state?
            match parser.base_url {
                Some(&Url { scheme: ref base_scheme, scheme_data: RelativeSchemeData(ref base),
                            ref query, .. })
                if scheme == *base_scheme => {
                    parse_relative_url(remaining, scheme, scheme_type, base, query, parser)
                },
                // FIXME: Should not have to use a made-up base URL.
                _ => parse_relative_url(remaining, scheme, scheme_type, &UrlRelativeSchemeData {
                    username: String::new(), password: None, host: Domain(String::new()),
                    port: None, default_port: None, path: Vec::new()
                }, &None, parser)
            }
        },
        RelativeScheme(..) => {
            match parser.base_url {
                Some(&Url { scheme: ref base_scheme, scheme_data: RelativeSchemeData(ref base),
                            ref query, .. })
                if scheme == *base_scheme && !remaining.starts_with("//") => {
                    try!(parser.parse_error(RelativeUrlWithScheme));
                    parse_relative_url(remaining, scheme, scheme_type, base, query, parser)
                },
                _ => parse_absolute_url(scheme, scheme_type, remaining, parser),
            }
        },
        NonRelativeScheme => {
            // Scheme data state
            let (scheme_data, remaining) = try!(parse_scheme_data(remaining, parser));
            let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
            Ok(Url { scheme: scheme, scheme_data: NonRelativeSchemeData(scheme_data),
                     query: query, fragment: fragment })
        }
    }
}


pub fn parse_scheme<'a>(input: &'a str, context: Context) -> Option<(String, &'a str)> {
    if input.is_empty() || !starts_with_ascii_alpha(input) {
        return None
    }
    for (i, c) in input.char_indices() {
        match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | '+' | '-' | '.' => (),
            ':' => return Some((
                input.slice_to(i).to_ascii_lower(),
                input.slice_from(i + 1),
            )),
            _ => return None,
        }
    }
    // EOF before ':'
    match context {
        SetterContext => Some((input.to_ascii_lower(), "")),
        UrlParserContext => None
    }
}


fn parse_absolute_url<'a>(scheme: String, scheme_type: SchemeType,
                          input: &'a str, parser: &UrlParser) -> ParseResult<Url> {
    // Authority first slash state
    let remaining = try!(skip_slashes(input, parser));
    // Authority state
    let (username, password, remaining) = try!(parse_userinfo(remaining, parser));
    // Host state
    let (host, port, default_port, remaining) = try!(parse_host(remaining, scheme_type, parser));
    let (path, remaining) = try!(parse_path_start(
        remaining, UrlParserContext, scheme_type, parser));
    let scheme_data = RelativeSchemeData(UrlRelativeSchemeData {
        username: username, password: password,
        host: host, port: port, default_port: default_port,
        path: path });
    let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
    Ok(Url { scheme: scheme, scheme_data: scheme_data, query: query, fragment: fragment })
}


fn parse_relative_url<'a>(input: &'a str, scheme: String, scheme_type: SchemeType,
                          base: &UrlRelativeSchemeData, base_query: &Option<String>,
                          parser: &UrlParser)
                          -> ParseResult<Url> {
    if input.is_empty() {
        return Ok(Url { scheme: scheme, scheme_data: RelativeSchemeData(base.clone()),
                        query: base_query.clone(), fragment: None })
    }
    match input.char_at(0) {
        '/' | '\\' => {
            // Relative slash state
            if input.len() > 1 && is_match!(input.char_at(1), '/' | '\\') {
                if input.char_at(1) == '\\' { try!(parser.parse_error(InvalidBackslash)) }
                if scheme_type == FileLikeRelativeScheme {
                    // File host state
                    let remaining = input.slice_from(2);
                    let (host, remaining) = if remaining.len() >= 2
                       && starts_with_ascii_alpha(remaining)
                       && is_match!(remaining.char_at(1), ':' | '|')
                       && (remaining.len() == 2
                           || is_match!(remaining.char_at(2),
                                         '/' | '\\' | '?' | '#'))
                    {
                        // Windows drive letter quirk
                        (Domain(String::new()), remaining)
                    } else {
                        try!(parse_file_host(remaining, parser))
                    };
                    let (path, remaining) = try!(parse_path_start(
                        remaining, UrlParserContext, scheme_type, parser));
                    let scheme_data = RelativeSchemeData(UrlRelativeSchemeData {
                        username: String::new(), password: None,
                        host: host, port: None, default_port: None, path: path
                    });
                    let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
                    Ok(Url { scheme: scheme, scheme_data: scheme_data,
                             query: query, fragment: fragment })
                } else {
                    parse_absolute_url(scheme, scheme_type, input, parser)
                }
            } else {
                // Relative path state
                let (path, remaining) = try!(parse_path(
                    &[], input.slice_from(1), UrlParserContext, scheme_type, parser));
                let scheme_data = RelativeSchemeData(if scheme_type == FileLikeRelativeScheme {
                    UrlRelativeSchemeData {
                        username: String::new(), password: None, host:
                        Domain(String::new()), port: None, default_port: None, path: path
                    }
                } else {
                    UrlRelativeSchemeData {
                        username: base.username.clone(),
                        password: base.password.clone(),
                        host: base.host.clone(),
                        port: base.port.clone(),
                        default_port: base.default_port.clone(),
                        path: path
                    }
                });
                let (query, fragment) = try!(
                    parse_query_and_fragment(remaining, parser));
                Ok(Url { scheme: scheme, scheme_data: scheme_data,
                         query: query, fragment: fragment })
            }
        },
        '?' => {
            let (query, fragment) = try!(parse_query_and_fragment(input, parser));
            Ok(Url { scheme: scheme, scheme_data: RelativeSchemeData(base.clone()),
                     query: query, fragment: fragment })
        },
        '#' => {
            let fragment = Some(try!(parse_fragment(input.slice_from(1), parser)));
            Ok(Url { scheme: scheme, scheme_data: RelativeSchemeData(base.clone()),
                     query: base_query.clone(), fragment: fragment })
        }
        _ => {
            let (scheme_data, remaining) = if scheme_type == FileLikeRelativeScheme
               && input.len() >= 2
               && starts_with_ascii_alpha(input)
               && is_match!(input.char_at(1), ':' | '|')
               && (input.len() == 2
                   || is_match!(input.char_at(2), '/' | '\\' | '?' | '#'))
            {
                // Windows drive letter quirk
                let (path, remaining) = try!(parse_path(
                    &[], input, UrlParserContext, scheme_type, parser));
                 (RelativeSchemeData(UrlRelativeSchemeData {
                    username: String::new(), password: None,
                    host: Domain(String::new()),
                    port: None,
                    default_port: None,
                    path: path
                }), remaining)
            } else {
                let base_path = base.path.slice_to(base.path.len() - 1);
                // Relative path state
                let (path, remaining) = try!(parse_path(
                    base_path, input, UrlParserContext, scheme_type, parser));
                (RelativeSchemeData(UrlRelativeSchemeData {
                    username: base.username.clone(),
                    password: base.password.clone(),
                    host: base.host.clone(),
                    port: base.port.clone(),
                    default_port: base.default_port.clone(),
                    path: path
                }), remaining)
            };
            let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
            Ok(Url { scheme: scheme, scheme_data: scheme_data,
                     query: query, fragment: fragment })
        }
    }
}


fn skip_slashes<'a>(input: &'a str, parser: &UrlParser) -> ParseResult<&'a str> {
    let first_non_slash = input.find(|c| !is_match!(c, '/' | '\\')).unwrap_or(input.len());
    if input.slice_to(first_non_slash) != "//" {
        try!(parser.parse_error(ExpectedTwoSlashes));
    }
    Ok(input.slice_from(first_non_slash))
}


fn parse_userinfo<'a>(input: &'a str, parser: &UrlParser)
                      -> ParseResult<(String, Option<String>, &'a str)> {
    let mut last_at = None;
    for (i, c) in input.char_indices() {
        match c {
            '@' => {
                if last_at.is_some() { try!(parser.parse_error(InvalidAtSymbolInUser)) }
                last_at = Some(i)
            },
            '/' | '\\' | '?' | '#' => break,
            _ => (),
        }
    }
    let (input, remaining) = match last_at {
        Some(at) => (input.slice_to(at), input.slice_from(at + 1)),
        None => return Ok((String::new(), None, input)),
    };

    let mut username = String::new();
    let mut password = None;
    for (i, c, next_i) in input.char_ranges() {
        match c {
            ':' => {
                password = Some(try!(parse_password(input.slice_from(i + 1), parser)));
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                // The spec says to use the default encode set,
                // but also replaces '@' by '%40' in an earlier step.
                utf8_percent_encode_to(input.slice(i, next_i),
                                    USERINFO_ENCODE_SET, &mut username);
            }
        }
    }
    Ok((username, password, remaining))
}


fn parse_password(input: &str, parser: &UrlParser) -> ParseResult<String> {
    let mut password = String::new();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                // The spec says to use the default encode set,
                // but also replaces '@' by '%40' in an earlier step.
                utf8_percent_encode_to(input.slice(i, next_i),
                                    USERINFO_ENCODE_SET, &mut password);
            }
        }
    }
    Ok(password)
}


pub fn parse_host<'a>(input: &'a str, scheme_type: SchemeType, parser: &UrlParser)
                          -> ParseResult<(Host, Option<u16>, Option<u16>, &'a str)> {
    let (host, remaining) = try!(parse_hostname(input, parser));
    let (port, default_port, remaining) = if remaining.starts_with(":") {
        try!(parse_port(remaining.slice_from(1), scheme_type, parser))
    } else {
        (None, scheme_type.default_port(), remaining)
    };
    Ok((host, port, default_port, remaining))
}


pub fn parse_hostname<'a>(input: &'a str, parser: &UrlParser)
                      -> ParseResult<(Host, &'a str)> {
    let mut inside_square_brackets = false;
    let mut host_input = String::new();
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            ':' if !inside_square_brackets => {
                end = i;
                break
            },
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            c => {
                match c {
                    '[' => inside_square_brackets = true,
                    ']' => inside_square_brackets = false,
                    _ => (),
                }
                host_input.push(c)
            }
        }
    }
    let host = try!(Host::parse(host_input.as_slice()));
    Ok((host, input.slice_from(end)))
}


pub fn parse_port<'a>(input: &'a str, scheme_type: SchemeType, parser: &UrlParser)
                      -> ParseResult<(Option<u16>, Option<u16>, &'a str)> {
    let mut port = 0;
    let mut has_any_digit = false;
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            '0'...'9' => {
                port = port * 10 + (c as u32 - '0' as u32);
                if port > ::std::u16::MAX as u32 {
                    return Err(InvalidPort)
                }
                has_any_digit = true;
            },
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => return Err(InvalidPort)
        }
    }
    let default_port = scheme_type.default_port();
    let mut port = Some(port as u16);
    if !has_any_digit || port == default_port {
        port = None;
    }
    return Ok((port, default_port, input.slice_from(end)))
}


fn parse_file_host<'a>(input: &'a str, parser: &UrlParser) -> ParseResult<(Host, &'a str)> {
    let mut host_input = String::new();
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => host_input.push(c)
        }
    }
    let host = if host_input.is_empty() {
        Domain(String::new())
    } else {
        try!(Host::parse(host_input.as_slice()))
    };
    Ok((host, input.slice_from(end)))
}


pub fn parse_standalone_path(input: &str, parser: &UrlParser)
                             -> ParseResult<(Vec<String>, Option<String>, Option<String>)> {
    if !input.starts_with("/") {
        if input.starts_with("\\") {
            try!(parser.parse_error(InvalidBackslash));
        } else {
            return Err(ExpectedInitialSlash)
        }
    }
    let (path, remaining) = try!(parse_path(
        &[], input.slice_from(1), UrlParserContext, RelativeScheme(0), parser));
    let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
    Ok((path, query, fragment))
}


pub fn parse_path_start<'a>(input: &'a str, context: Context, scheme_type: SchemeType,
                            parser: &UrlParser)
                            -> ParseResult<(Vec<String>, &'a str)> {
    let mut i = 0;
    // Relative path start state
    if !input.is_empty() {
        match input.char_at(0) {
            '/' => i = 1,
            '\\' => {
                try!(parser.parse_error(InvalidBackslash));
                i = 1;
            },
            _ => ()
        }
    }
    parse_path(&[], input.slice_from(i), context, scheme_type, parser)
}


fn parse_path<'a>(base_path: &[String], input: &'a str, context: Context,
                  scheme_type: SchemeType, parser: &UrlParser)
                  -> ParseResult<(Vec<String>, &'a str)> {
    // Relative path state
    let mut path = base_path.to_vec();
    let mut iter = input.char_ranges();
    let mut end;
    loop {
        let mut path_part = String::new();
        let mut ends_with_slash = false;
        end = input.len();
        for (i, c, next_i) in iter {
            match c {
                '/' => {
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '\\' => {
                    try!(parser.parse_error(InvalidBackslash));
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '?' | '#' if context == UrlParserContext => {
                    end = i;
                    break
                },
                '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
                _ => {
                    try!(check_url_code_point(input, i, c, parser));
                    utf8_percent_encode_to(input.slice(i, next_i),
                                        DEFAULT_ENCODE_SET, &mut path_part);
                }
            }
        }
        match path_part.as_slice() {
            ".." | ".%2e" | ".%2E" | "%2e." | "%2E." |
            "%2e%2e" | "%2E%2e" | "%2e%2E" | "%2E%2E" => {
                path.pop();
                if !ends_with_slash {
                    path.push(String::new());
                }
            },
            "." | "%2e" | "%2E" => {
                if !ends_with_slash {
                    path.push(String::new());
                }
            },
            _ => {
                if scheme_type == FileLikeRelativeScheme
                   && path.is_empty()
                   && path_part.len() == 2
                   && starts_with_ascii_alpha(path_part.as_slice())
                   && path_part.as_slice().char_at(1) == '|' {
                    // Windows drive letter quirk
                    unsafe {
                        path_part.as_mut_vec()[1] = b':'
                    }
                }
                path.push(path_part)
            }
        }
        if !ends_with_slash {
            break
        }
    }
    Ok((path, input.slice_from(end)))
}


fn parse_scheme_data<'a>(input: &'a str, parser: &UrlParser)
                         -> ParseResult<(String, &'a str)> {
    let mut scheme_data = String::new();
    let mut end = input.len();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                utf8_percent_encode_to(input.slice(i, next_i),
                                    SIMPLE_ENCODE_SET, &mut scheme_data);
            }
        }
    }
    Ok((scheme_data, input.slice_from(end)))
}


fn parse_query_and_fragment(input: &str, parser: &UrlParser)
                            -> ParseResult<(Option<String>, Option<String>)> {
    if input.is_empty() {
        return Ok((None, None))
    }
    match input.char_at(0) {
        '#' => Ok((None, Some(try!(parse_fragment(input.slice_from(1), parser))))),
        '?' => {
            let (query, remaining) = try!(parse_query(
                input.slice_from(1), UrlParserContext, parser));
            let fragment = match remaining {
                Some(remaining) => Some(try!(parse_fragment(remaining, parser))),
                None => None
            };
            Ok((Some(query), fragment))
        },
        _ => panic!("Programming error. parse_query_and_fragment() should not \
                    have been called with input \"{}\"", input)
    }
}


pub fn parse_query<'a>(input: &'a str, context: Context, parser: &UrlParser)
                   -> ParseResult<(String, Option<&'a str>)> {
    let mut query = String::new();
    let mut remaining = None;
    for (i, c) in input.char_indices() {
        match c {
            '#' if context == UrlParserContext => {
                remaining = Some(input.slice_from(i + 1));
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                query.push(c);
            }
        }
    }
    let encoded;
    let query_bytes = match parser.query_encoding_override {
        Some(encoding) => {
            encoded = encoding.encode(query.as_slice(), encoding::EncoderTrap::Replace).unwrap();
            encoded.as_slice()
        },
        None => query.as_bytes()  // UTF-8
    };
    ;
    Ok((percent_encode(query_bytes.as_slice(), QUERY_ENCODE_SET), remaining))
}


pub fn parse_fragment<'a>(input: &'a str, parser: &UrlParser) -> ParseResult<String> {
    let mut fragment = String::new();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '\t' | '\n' | '\r' => try!(parser.parse_error(InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                utf8_percent_encode_to(input.slice(i, next_i),
                                    SIMPLE_ENCODE_SET, &mut fragment);
            }
        }
    }
    Ok(fragment)
}


#[inline]
pub fn starts_with_ascii_alpha(string: &str) -> bool {
    match string.char_at(0) {
        'a'...'z' | 'A'...'Z' => true,
        _ => false,
    }
}

#[inline]
fn is_ascii_hex_digit(byte: u8) -> bool {
    match byte {
        b'a'...b'f' | b'A'...b'F' | b'0'...b'9' => true,
        _ => false,
    }
}

#[inline]
fn starts_with_2_hex(input: &str) -> bool {
    input.len() >= 2
    && is_ascii_hex_digit(input.as_bytes()[0])
    && is_ascii_hex_digit(input.as_bytes()[1])
}

#[inline]
fn is_url_code_point(c: char) -> bool {
    match c {
        'a'...'z' |
        'A'...'Z' |
        '0'...'9' |
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '-' |
        '.' | '/' | ':' | ';' | '=' | '?' | '@' | '_' | '~' |
        '\u00A0'...'\uD7FF' | '\uE000'...'\uFDCF' | '\uFDF0'...'\uFFFD' |
        '\U00010000'...'\U0001FFFD' | '\U00020000'...'\U0002FFFD' |
        '\U00030000'...'\U0003FFFD' | '\U00040000'...'\U0004FFFD' |
        '\U00050000'...'\U0005FFFD' | '\U00060000'...'\U0006FFFD' |
        '\U00070000'...'\U0007FFFD' | '\U00080000'...'\U0008FFFD' |
        '\U00090000'...'\U0009FFFD' | '\U000A0000'...'\U000AFFFD' |
        '\U000B0000'...'\U000BFFFD' | '\U000C0000'...'\U000CFFFD' |
        '\U000D0000'...'\U000DFFFD' | '\U000E1000'...'\U000EFFFD' |
        '\U000F0000'...'\U000FFFFD' | '\U00100000'...'\U0010FFFD' => true,
        _ => false
    }
}

// Non URL code points:
// U+0000 to U+0020 (space)
// " # % < > [ \ ] ^ ` { | }
// U+007F to U+009F
// surrogates
// U+FDD0 to U+FDEF
// Last two of each plane: U+__FFFE to U+__FFFF for __ in 00 to 10 hex


pub trait StrCharRanges<'a> {
    fn char_ranges(&self) -> CharRanges<'a>;
}


impl<'a> StrCharRanges<'a> for &'a str {
    #[inline]
    fn char_ranges(&self) -> CharRanges<'a> {
        CharRanges { slice: *self, position: 0 }
    }
}

pub struct CharRanges<'a> {
    slice: &'a str,
    position: uint,
}

impl<'a> Iterator<(uint, char, uint)> for CharRanges<'a> {
    #[inline]
    fn next(&mut self) -> Option<(uint, char, uint)> {
        if self.position == self.slice.len() {
            None
        } else {
            let position = self.position;
            let CharRange { ch, next } = self.slice.char_range_at(position);
            self.position = next;
            Some((position, ch, next))
        }
    }
}

#[inline]
fn check_url_code_point(input: &str, i: uint, c: char, parser: &UrlParser)
                        -> ParseResult<()> {
    if c == '%' {
        if !starts_with_2_hex(input.slice_from(i + 1)) {
            try!(parser.parse_error(InvalidPercentEncoded));
        }
    } else if !is_url_code_point(c) {
        try!(parser.parse_error(NonUrlCodePoint));
    }
    Ok(())
}

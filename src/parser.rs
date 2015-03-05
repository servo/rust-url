// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ascii::AsciiExt;
use std::error::Error;
use std::fmt::{self, Formatter};

use super::{UrlParser, Url, SchemeData, RelativeSchemeData, Host, SchemeType};
use percent_encoding::{
    utf8_percent_encode_to, percent_encode,
    SIMPLE_ENCODE_SET, DEFAULT_ENCODE_SET, USERINFO_ENCODE_SET, QUERY_ENCODE_SET
};


pub type ParseResult<T> = Result<T, ParseError>;


macro_rules! simple_enum_error {
    ($($name: ident => $description: expr,)+) => {
        /// Errors that can occur during parsing.
        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        pub enum ParseError {
            $(
                $name,
            )+
        }

        impl Error for ParseError {
            fn description(&self) -> &str {
                match *self {
                    $(
                        ParseError::$name => $description,
                    )+
                }
            }
        }
    }
}

simple_enum_error! {
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
    CannotSetJavascriptFragment => "cannot set fragment on javascript: URL",
    CannotSetPortWithFileLikeScheme => "cannot set port with file-like scheme",
    CannotSetUsernameWithNonRelativeScheme => "cannot set username with non-relative scheme",
    CannotSetPasswordWithNonRelativeScheme => "cannot set password with non-relative scheme",
    CannotSetHostPortWithNonRelativeScheme => "cannot set host and port with non-relative scheme",
    CannotSetHostWithNonRelativeScheme => "cannot set host with non-relative scheme",
    CannotSetPortWithNonRelativeScheme => "cannot set port with non-relative scheme",
    CannotSetPathWithNonRelativeScheme => "cannot set path with non-relative scheme",
}

impl fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.description().fmt(fmt)
    }
}

/// This is called on non-fatal parse errors.
///
/// The handler can choose to continue or abort parsing by returning Ok() or Err(), respectively.
/// See the `UrlParser::error_handler` method.
///
/// FIXME: make this a by-ref closure when that’s supported.
pub type ErrorHandler = fn(reason: ParseError) -> ParseResult<()>;


#[derive(PartialEq, Eq)]
pub enum Context {
    UrlParser,
    Setter,
}


pub fn parse_url(input: &str, parser: &UrlParser) -> ParseResult<Url> {
    let input = input.trim_matches(&[' ', '\t', '\n', '\r', '\x0C'][..]);
    let (scheme, remaining) = match parse_scheme(input, Context::UrlParser) {
        Some((scheme, remaining)) => (scheme, remaining),
        // No-scheme state
        None => return match parser.base_url {
            Some(&Url { ref scheme, scheme_data: SchemeData::Relative(ref base),
                        ref query, .. }) => {
                let scheme_type = parser.get_scheme_type(&scheme);
                parse_relative_url(input, scheme.clone(), scheme_type, base, query, parser)
            },
            Some(_) => Err(ParseError::RelativeUrlWithNonRelativeBase),
            None => Err(ParseError::RelativeUrlWithoutBase),
        },
    };
    let scheme_type = parser.get_scheme_type(&scheme);
    match scheme_type {
        SchemeType::FileLike => {
            // Relative state?
            match parser.base_url {
                Some(&Url { scheme: ref base_scheme, scheme_data: SchemeData::Relative(ref base),
                            ref query, .. })
                if scheme == *base_scheme => {
                    parse_relative_url(remaining, scheme, scheme_type, base, query, parser)
                },
                // FIXME: Should not have to use a made-up base URL.
                _ => parse_relative_url(remaining, scheme, scheme_type, &RelativeSchemeData {
                    username: String::new(), password: None, host: Host::Domain(String::new()),
                    port: None, default_port: None, path: Vec::new()
                }, &None, parser)
            }
        },
        SchemeType::Relative(..) => {
            match parser.base_url {
                Some(&Url { scheme: ref base_scheme, scheme_data: SchemeData::Relative(ref base),
                            ref query, .. })
                if scheme == *base_scheme && !remaining.starts_with("//") => {
                    try!(parser.parse_error(ParseError::RelativeUrlWithScheme));
                    parse_relative_url(remaining, scheme, scheme_type, base, query, parser)
                },
                _ => parse_absolute_url(scheme, scheme_type, remaining, parser),
            }
        },
        SchemeType::NonRelative => {
            // Scheme data state
            let (scheme_data, remaining) = try!(parse_scheme_data(remaining, parser));
            let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
            Ok(Url { scheme: scheme, scheme_data: SchemeData::NonRelative(scheme_data),
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
                input[..i].to_ascii_lowercase(),
                &input[i + 1..],
            )),
            _ => return None,
        }
    }
    // EOF before ':'
    match context {
        Context::Setter => Some((input.to_ascii_lowercase(), "")),
        Context::UrlParser => None
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
        remaining, Context::UrlParser, scheme_type, parser));
    let scheme_data = SchemeData::Relative(RelativeSchemeData {
        username: username, password: password,
        host: host, port: port, default_port: default_port,
        path: path });
    let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
    Ok(Url { scheme: scheme, scheme_data: scheme_data, query: query, fragment: fragment })
}


fn parse_relative_url<'a>(input: &'a str, scheme: String, scheme_type: SchemeType,
                          base: &RelativeSchemeData, base_query: &Option<String>,
                          parser: &UrlParser)
                          -> ParseResult<Url> {
    let mut chars = input.chars();
    match chars.next() {
        Some('/') | Some('\\') => {
            let ch = chars.next();
            // Relative slash state
            if matches!(ch, Some('/') | Some('\\')) {
                if ch == Some('\\') {
                    try!(parser.parse_error(ParseError::InvalidBackslash))
                }
                if scheme_type == SchemeType::FileLike {
                    // File host state
                    let remaining = &input[2..];
                    let (host, remaining) = if remaining.len() >= 2
                       && starts_with_ascii_alpha(remaining)
                       && matches!(remaining.as_bytes()[1], b':' | b'|')
                       && (remaining.len() == 2
                           || matches!(remaining.as_bytes()[2],
                                         b'/' | b'\\' | b'?' | b'#'))
                    {
                        // Windows drive letter quirk
                        (Host::Domain(String::new()), remaining)
                    } else {
                        try!(parse_file_host(remaining, parser))
                    };
                    let (path, remaining) = try!(parse_path_start(
                        remaining, Context::UrlParser, scheme_type, parser));
                    let scheme_data = SchemeData::Relative(RelativeSchemeData {
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
                    &[], &input[1..], Context::UrlParser, scheme_type, parser));
                let scheme_data = SchemeData::Relative(if scheme_type == SchemeType::FileLike {
                    RelativeSchemeData {
                        username: String::new(), password: None, host:
                        Host::Domain(String::new()), port: None, default_port: None, path: path
                    }
                } else {
                    RelativeSchemeData {
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
        Some('?') => {
            let (query, fragment) = try!(parse_query_and_fragment(input, parser));
            Ok(Url { scheme: scheme, scheme_data: SchemeData::Relative(base.clone()),
                     query: query, fragment: fragment })
        },
        Some('#') => {
            let fragment = Some(try!(parse_fragment(&input[1..], parser)));
            Ok(Url { scheme: scheme, scheme_data: SchemeData::Relative(base.clone()),
                     query: base_query.clone(), fragment: fragment })
        }
        None => {
            Ok(Url { scheme: scheme, scheme_data: SchemeData::Relative(base.clone()),
                     query: base_query.clone(), fragment: None })
        }
        _ => {
            let (scheme_data, remaining) = if scheme_type == SchemeType::FileLike
               && input.len() >= 2
               && starts_with_ascii_alpha(input)
               && matches!(input.as_bytes()[1], b':' | b'|')
               && (input.len() == 2
                   || matches!(input.as_bytes()[2], b'/' | b'\\' | b'?' | b'#'))
            {
                // Windows drive letter quirk
                let (path, remaining) = try!(parse_path(
                    &[], input, Context::UrlParser, scheme_type, parser));
                 (SchemeData::Relative(RelativeSchemeData {
                    username: String::new(), password: None,
                    host: Host::Domain(String::new()),
                    port: None,
                    default_port: None,
                    path: path
                }), remaining)
            } else {
                let base_path = &base.path[..base.path.len() - 1];
                // Relative path state
                let (path, remaining) = try!(parse_path(
                    base_path, input, Context::UrlParser, scheme_type, parser));
                (SchemeData::Relative(RelativeSchemeData {
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
    let first_non_slash = input.find(|c| !matches!(c, '/' | '\\')).unwrap_or(input.len());
    if &input[..first_non_slash] != "//" {
        try!(parser.parse_error(ParseError::ExpectedTwoSlashes));
    }
    Ok(&input[first_non_slash..])
}


fn parse_userinfo<'a>(input: &'a str, parser: &UrlParser)
                      -> ParseResult<(String, Option<String>, &'a str)> {
    let mut last_at = None;
    for (i, c) in input.char_indices() {
        match c {
            '@' => {
                if last_at.is_some() {
                    try!(parser.parse_error(ParseError::InvalidAtSymbolInUser))
                }
                last_at = Some(i)
            },
            '/' | '\\' | '?' | '#' => break,
            _ => (),
        }
    }
    let (input, remaining) = match last_at {
        Some(at) => (&input[..at], &input[at + 1..]),
        None => return Ok((String::new(), None, input)),
    };

    let mut username = String::new();
    let mut password = None;
    for (i, c, next_i) in input.char_ranges() {
        match c {
            ':' => {
                password = Some(try!(parse_password(&input[i + 1..], parser)));
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                // The spec says to use the default encode set,
                // but also replaces '@' by '%40' in an earlier step.
                utf8_percent_encode_to(&input[i..next_i],
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
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                // The spec says to use the default encode set,
                // but also replaces '@' by '%40' in an earlier step.
                utf8_percent_encode_to(&input[i..next_i],
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
        try!(parse_port(&remaining[1..], scheme_type, parser))
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
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
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
    let host = try!(Host::parse(&host_input));
    Ok((host, &input[end..]))
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
                    return Err(ParseError::InvalidPort)
                }
                has_any_digit = true;
            },
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => return Err(ParseError::InvalidPort)
        }
    }
    let default_port = scheme_type.default_port();
    let mut port = Some(port as u16);
    if !has_any_digit || port == default_port {
        port = None;
    }
    return Ok((port, default_port, &input[end..]))
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
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => host_input.push(c)
        }
    }
    let host = if host_input.is_empty() {
        Host::Domain(String::new())
    } else {
        try!(Host::parse(&host_input))
    };
    Ok((host, &input[end..]))
}


pub fn parse_standalone_path(input: &str, parser: &UrlParser)
                             -> ParseResult<(Vec<String>, Option<String>, Option<String>)> {
    if !input.starts_with("/") {
        if input.starts_with("\\") {
            try!(parser.parse_error(ParseError::InvalidBackslash));
        } else {
            return Err(ParseError::ExpectedInitialSlash)
        }
    }
    let (path, remaining) = try!(parse_path(
        &[], &input[1..], Context::UrlParser, SchemeType::Relative(0), parser));
    let (query, fragment) = try!(parse_query_and_fragment(remaining, parser));
    Ok((path, query, fragment))
}


pub fn parse_path_start<'a>(input: &'a str, context: Context, scheme_type: SchemeType,
                            parser: &UrlParser)
                            -> ParseResult<(Vec<String>, &'a str)> {
    let mut i = 0;
    // Relative path start state
    match input.chars().next() {
        Some('/') => i = 1,
        Some('\\') => {
            try!(parser.parse_error(ParseError::InvalidBackslash));
            i = 1;
        },
        _ => ()
    }
    parse_path(&[], &input[i..], context, scheme_type, parser)
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
        while let Some((i, c, next_i)) = iter.next() {
            match c {
                '/' => {
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '\\' => {
                    try!(parser.parse_error(ParseError::InvalidBackslash));
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '?' | '#' if context == Context::UrlParser => {
                    end = i;
                    break
                },
                '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
                _ => {
                    try!(check_url_code_point(input, i, c, parser));
                    utf8_percent_encode_to(&input[i..next_i],
                                        DEFAULT_ENCODE_SET, &mut path_part);
                }
            }
        }
        match &*path_part {
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
                if scheme_type == SchemeType::FileLike
                   && path.is_empty()
                   && path_part.len() == 2
                   && starts_with_ascii_alpha(&path_part)
                   && path_part.as_bytes()[1] == b'|' {
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
    Ok((path, &input[end..]))
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
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                utf8_percent_encode_to(&input[i..next_i],
                                    SIMPLE_ENCODE_SET, &mut scheme_data);
            }
        }
    }
    Ok((scheme_data, &input[end..]))
}


fn parse_query_and_fragment(input: &str, parser: &UrlParser)
                            -> ParseResult<(Option<String>, Option<String>)> {
    match input.chars().next() {
        Some('#') => Ok((None, Some(try!(parse_fragment(&input[1..], parser))))),
        Some('?') => {
            let (query, remaining) = try!(parse_query(
                &input[1..], Context::UrlParser, parser));
            let fragment = match remaining {
                Some(remaining) => Some(try!(parse_fragment(remaining, parser))),
                None => None
            };
            Ok((Some(query), fragment))
        },
        None => Ok((None, None)),
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
            '#' if context == Context::UrlParser => {
                remaining = Some(&input[i + 1..]);
                break
            },
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                query.push(c);
            }
        }
    }

    let query_bytes = parser.query_encoding_override.encode(&query);
    Ok((percent_encode(&query_bytes, QUERY_ENCODE_SET), remaining))
}


pub fn parse_fragment<'a>(input: &'a str, parser: &UrlParser) -> ParseResult<String> {
    let mut fragment = String::new();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '\t' | '\n' | '\r' => try!(parser.parse_error(ParseError::InvalidCharacter)),
            _ => {
                try!(check_url_code_point(input, i, c, parser));
                utf8_percent_encode_to(&input[i..next_i],
                                    SIMPLE_ENCODE_SET, &mut fragment);
            }
        }
    }
    Ok(fragment)
}


#[inline]
pub fn starts_with_ascii_alpha(string: &str) -> bool {
    matches!(string.as_bytes()[0], b'a'...b'z' | b'A'...b'Z')
}

#[inline]
fn is_ascii_hex_digit(byte: u8) -> bool {
    matches!(byte, b'a'...b'f' | b'A'...b'F' | b'0'...b'9')
}

#[inline]
fn starts_with_2_hex(input: &str) -> bool {
    input.len() >= 2
    && is_ascii_hex_digit(input.as_bytes()[0])
    && is_ascii_hex_digit(input.as_bytes()[1])
}

#[inline]
fn is_url_code_point(c: char) -> bool {
    matches!(c,
        'a'...'z' |
        'A'...'Z' |
        '0'...'9' |
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '-' |
        '.' | '/' | ':' | ';' | '=' | '?' | '@' | '_' | '~' |
        '\u{A0}'...'\u{D7FF}' | '\u{E000}'...'\u{FDCF}' | '\u{FDF0}'...'\u{FFFD}' |
        '\u{10000}'...'\u{1FFFD}' | '\u{20000}'...'\u{2FFFD}' |
        '\u{30000}'...'\u{3FFFD}' | '\u{40000}'...'\u{4FFFD}' |
        '\u{50000}'...'\u{5FFFD}' | '\u{60000}'...'\u{6FFFD}' |
        '\u{70000}'...'\u{7FFFD}' | '\u{80000}'...'\u{8FFFD}' |
        '\u{90000}'...'\u{9FFFD}' | '\u{A0000}'...'\u{AFFFD}' |
        '\u{B0000}'...'\u{BFFFD}' | '\u{C0000}'...'\u{CFFFD}' |
        '\u{D0000}'...'\u{DFFFD}' | '\u{E1000}'...'\u{EFFFD}' |
        '\u{F0000}'...'\u{FFFFD}' | '\u{100000}'...'\u{10FFFD}')
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
    position: usize,
}

impl<'a> Iterator for CharRanges<'a> {
    type Item = (usize, char, usize);

    #[inline]
    fn next(&mut self) -> Option<(usize, char, usize)> {
        match self.slice[self.position..].chars().next() {
            Some(ch) => {
                let position = self.position;
                self.position = position + ch.len_utf8();
                Some((position, ch, position + ch.len_utf8()))
            }
            None => None,
        }
    }
}

#[inline]
fn check_url_code_point(input: &str, i: usize, c: char, parser: &UrlParser)
                        -> ParseResult<()> {
    if c == '%' {
        if !starts_with_2_hex(&input[i + 1..]) {
            try!(parser.parse_error(ParseError::InvalidPercentEncoded));
        }
    } else if !is_url_code_point(c) {
        try!(parser.parse_error(ParseError::NonUrlCodePoint));
    }
    Ok(())
}

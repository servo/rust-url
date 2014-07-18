// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::ascii::StrAsciiExt;
use std::str::CharRange;

use encoding;
use encoding::EncodingRef;

use super::{
    ParseResult, ErrorHandler, Url, RelativeSchemeData, OtherSchemeData, Host, Domain,
    utf8_percent_encode, percent_encode};
use encode_sets::{SIMPLE_ENCODE_SET, DEFAULT_ENCODE_SET, USERINFO_ENCODE_SET, QUERY_ENCODE_SET};


macro_rules! is_match(
    ($value:expr, $($pattern:pat)|+) => (
        match $value { $($pattern)|+ => true, _ => false }
    );
)

#[deriving(PartialEq, Eq)]
pub enum Context {
    UrlParserContext,
    SetterContext,
}

#[deriving(PartialEq, Eq)]
pub enum SchemeType {
    FileScheme,
    NonFileScheme,
}


pub fn parse_url(input: &str, base_url: Option<&Url>, encoding_override: Option<EncodingRef>,
                 parse_error: ErrorHandler)
                 -> ParseResult<Url> {
    let input = input.trim_chars(&[' ', '\t', '\n', '\r', '\x0C']);
    let (scheme, remaining) = match parse_scheme(input, UrlParserContext) {
        Some((scheme, remaining)) => (scheme, remaining),
        // No-scheme state
        None => return match base_url {
            Some(&Url { ref scheme, scheme_data: RelativeSchemeData(ref base),
                        ref query, .. }) => {
                parse_relative_url(input, scheme.clone(), base, query,
                                   encoding_override, parse_error)
            },
            Some(_) => Err("Relative URL with a non-relative base"),
            None => Err("Relative URL without a base"),
        },
    };
    if scheme.as_slice() == "file" {
        // Relative state?
        match base_url {
            Some(&Url { scheme: ref base_scheme, scheme_data: RelativeSchemeData(ref base),
                        ref query, .. })
            if scheme == *base_scheme => {
                parse_relative_url(remaining, scheme, base, query, encoding_override, parse_error)
            },
            // FIXME: Should not have to use a made-up base URL.
            _ => parse_relative_url(remaining, scheme, &RelativeSchemeData {
                username: String::new(), password: None, host: Domain(String::new()),
                port: String::new(), path: Vec::new()
            }, &None, encoding_override, parse_error)
        }
    } else if is_relative_scheme(scheme.as_slice()) {
        match base_url {
            Some(&Url { scheme: ref base_scheme, scheme_data: RelativeSchemeData(ref base),
                        ref query, .. })
            if scheme == *base_scheme && !remaining.starts_with("//") => {
                try!(parse_error("Relative URL with a scheme"));
                parse_relative_url(remaining, scheme, base, query, encoding_override, parse_error)
            },
            _ => parse_absolute_url(scheme, remaining, encoding_override, parse_error),
        }
    } else {
        // Scheme data state
        let (scheme_data, remaining) = try!(parse_scheme_data(remaining, parse_error));
        let (query, fragment) = try!(parse_query_and_fragment(
            remaining, encoding_override, parse_error));
        Ok(Url { scheme: scheme, scheme_data: OtherSchemeData(scheme_data),
                 query: query, fragment: fragment, encoding_override: encoding_override })
    }
}


pub fn parse_scheme<'a>(input: &'a str, context: Context) -> Option<(String, &'a str)> {
    if input.is_empty() || !starts_with_ascii_alpha(input) {
        return None
    }
    for (i, c) in input.char_indices() {
        match c {
            'a'..'z' | 'A'..'Z' | '0'..'9' | '+' | '-' | '.' => (),
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


fn parse_absolute_url<'a>(scheme: String, input: &'a str, encoding_override: Option<EncodingRef>,
                          parse_error: ErrorHandler)
                          -> ParseResult<Url> {
    // Authority first slash state
    let remaining = try!(skip_slashes(input, parse_error));
    // Authority state
    let (username, password, remaining) = try!(parse_userinfo(remaining, parse_error));
    // Host state
    let (host, port, remaining) = try!(parse_host(remaining, scheme.as_slice(), parse_error));
    let (path, remaining) = try!(parse_path_start(
        remaining, UrlParserContext, NonFileScheme, parse_error));
    let scheme_data = RelativeSchemeData(RelativeSchemeData {
        username: username, password: password, host: host, port: port, path: path });
    let (query, fragment) = try!(parse_query_and_fragment(
        remaining, encoding_override, parse_error));
    Ok(Url { scheme: scheme, scheme_data: scheme_data, query: query, fragment: fragment,
             encoding_override: encoding_override })
}


fn parse_relative_url<'a>(input: &'a str, scheme: String, base: &RelativeSchemeData,
                          base_query: &Option<String>, encoding_override: Option<EncodingRef>,
                          parse_error: ErrorHandler)
                          -> ParseResult<Url> {
    if input.is_empty() {
        return Ok(Url { scheme: scheme, scheme_data: RelativeSchemeData(base.clone()),
                        query: base_query.clone(), fragment: None,
                        encoding_override: encoding_override })
    }
    let scheme_type = if scheme.as_slice() == "file" { FileScheme } else { NonFileScheme };
    match input.char_at(0) {
        '/' | '\\' => {
            // Relative slash state
            if input.len() > 1 && is_match!(input.char_at(1), '/' | '\\') {
                if input.char_at(1) == '\\' { try!(parse_error("backslash")) }
                if scheme_type == FileScheme {
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
                        try!(parse_file_host(remaining, parse_error))
                    };
                    let (path, remaining) = try!(parse_path_start(
                        remaining, UrlParserContext,
                        scheme_type, parse_error));
                    let scheme_data = RelativeSchemeData(RelativeSchemeData {
                        username: String::new(), password: None,
                        host: host, port: String::new(), path: path
                    });
                    let (query, fragment) = try!(parse_query_and_fragment(
                        remaining, encoding_override, parse_error));
                    Ok(Url { scheme: scheme, scheme_data: scheme_data,
                             query: query, fragment: fragment,
                             encoding_override: encoding_override })
                } else {
                    parse_absolute_url(scheme, input, encoding_override, parse_error)
                }
            } else {
                // Relative path state
                let (path, remaining) = try!(parse_path(
                    [], input.slice_from(1), UrlParserContext,
                    scheme_type, parse_error));
                let scheme_data = RelativeSchemeData(if scheme_type == FileScheme {
                    RelativeSchemeData {
                        username: String::new(), password: None, host:
                        Domain(String::new()), port: String::new(), path: path
                    }
                } else {
                    RelativeSchemeData {
                        username: base.username.clone(),
                        password: base.password.clone(),
                        host: base.host.clone(),
                        port: base.port.clone(),
                        path: path
                    }
                });
                let (query, fragment) = try!(
                    parse_query_and_fragment(
                        remaining, encoding_override, parse_error));
                Ok(Url { scheme: scheme, scheme_data: scheme_data,
                         query: query, fragment: fragment,
                         encoding_override: encoding_override })
            }
        },
        '?' => {
            let (query, fragment) = try!(parse_query_and_fragment(
                input, encoding_override, parse_error));
            Ok(Url { scheme: scheme, scheme_data: RelativeSchemeData(base.clone()),
                     query: query, fragment: fragment,
                     encoding_override: encoding_override })
        },
        '#' => {
            let fragment = Some(try!(parse_fragment(input.slice_from(1), parse_error)));
            Ok(Url { scheme: scheme, scheme_data: RelativeSchemeData(base.clone()),
                     query: base_query.clone(), fragment: fragment,
                     encoding_override: encoding_override })
        }
        _ => {
            let (scheme_data, remaining) = if scheme_type == FileScheme
               && input.len() >= 2
               && starts_with_ascii_alpha(input)
               && is_match!(input.char_at(1), ':' | '|')
               && (input.len() == 2
                   || is_match!(input.char_at(2), '/' | '\\' | '?' | '#'))
            {
                // Windows drive letter quirk
                let (path, remaining) = try!(parse_path(
                    [], input, UrlParserContext,
                    scheme_type, parse_error));
                 (RelativeSchemeData(RelativeSchemeData {
                    username: String::new(), password: None,
                    host: Domain(String::new()),
                    port: String::new(),
                    path: path
                }), remaining)
            } else {
                let base_path = base.path.slice_to(base.path.len() - 1);
                // Relative path state
                let (path, remaining) = try!(parse_path(
                    base_path, input, UrlParserContext,
                    scheme_type, parse_error));
                (RelativeSchemeData(RelativeSchemeData {
                    username: base.username.clone(),
                    password: base.password.clone(),
                    host: base.host.clone(),
                    port: base.port.clone(),
                    path: path
                }), remaining)
            };
            let (query, fragment) = try!(parse_query_and_fragment(
                remaining, encoding_override, parse_error));
            Ok(Url { scheme: scheme, scheme_data: scheme_data,
                     query: query, fragment: fragment,
                     encoding_override: encoding_override })
        }
    }
}


fn skip_slashes<'a>(input: &'a str, parse_error: ErrorHandler) -> ParseResult<&'a str> {
    let first_non_slash = input.find(|c| !is_match!(c, '/' | '\\')).unwrap_or(input.len());
    if input.slice_to(first_non_slash) != "//" {
        try!(parse_error("Expected two slashes"));
    }
    Ok(input.slice_from(first_non_slash))
}


fn parse_userinfo<'a>(input: &'a str, parse_error: ErrorHandler)
                      -> ParseResult<(String, Option<String>, &'a str)> {
    let mut last_at = None;
    for (i, c) in input.char_indices() {
        match c {
            '@' => {
                if last_at.is_some() { try!(parse_error("@ in userinfo")) }
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
                password = Some(try!(parse_password(input.slice_from(i + 1), parse_error)));
                break
            },
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => {
                try!(check_url_code_point(input, i, c, parse_error));
                // The spec says to use the default encode set,
                // but also replaces '@' by '%40' in an earlier step.
                utf8_percent_encode(input.slice(i, next_i),
                                    USERINFO_ENCODE_SET, &mut username);
            }
        }
    }
    Ok((username, password, remaining))
}


fn parse_password(input: &str, parse_error: ErrorHandler) -> ParseResult<String> {
    let mut password = String::new();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => {
                try!(check_url_code_point(input, i, c, parse_error));
                // The spec says to use the default encode set,
                // but also replaces '@' by '%40' in an earlier step.
                utf8_percent_encode(input.slice(i, next_i),
                                    USERINFO_ENCODE_SET, &mut password);
            }
        }
    }
    Ok(password)
}


pub fn parse_host<'a>(input: &'a str, scheme: &str, parse_error: ErrorHandler)
                          -> ParseResult<(Host, String, &'a str)> {
    let (host, remaining) = try!(parse_hostname(input, parse_error));
    let (port, remaining) = if remaining.starts_with(":") {
        try!(parse_port(remaining.slice_from(1), scheme, parse_error))
    } else {
        (String::new(), remaining)
    };
    Ok((host, port, remaining))
}


pub fn parse_hostname<'a>(input: &'a str, parse_error: ErrorHandler)
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
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            c => {
                match c {
                    '[' => inside_square_brackets = true,
                    ']' => inside_square_brackets = false,
                    _ => (),
                }
                host_input.push_char(c)
            }
        }
    }
    let host = try!(Host::parse(host_input.as_slice()));
    Ok((host, input.slice_from(end)))
}


pub fn parse_port<'a>(input: &'a str, scheme: &str, parse_error: ErrorHandler)
                  -> ParseResult<(String, &'a str)> {
    let mut port = String::new();
    let mut has_initial_zero = false;
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            '1'..'9' => port.push_char(c),
            '0' => {
                if port.is_empty() {
                    has_initial_zero = true
                } else {
                    port.push_char(c)
                }
            },
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => return Err("Invalid port number")
        }
    }
    if port.is_empty() && has_initial_zero {
        port.push_str("0")
    }
    match (scheme, port.as_slice()) {
        ("ftp", "21") | ("gopher", "70") | ("http", "80") |
        ("https", "443") | ("ws", "80") | ("wss", "443")
        => port.truncate(0),
        _ => (),
    }
    return Ok((port, input.slice_from(end)))
}


fn parse_file_host<'a>(input: &'a str, parse_error: ErrorHandler) -> ParseResult<(Host, &'a str)> {
    let mut host_input = String::new();
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => host_input.push_char(c)
        }
    }
    let host = if host_input.is_empty() {
        Domain(String::new())
    } else {
        try!(Host::parse(host_input.as_slice()))
    };
    Ok((host, input.slice_from(end)))
}


pub fn parse_path_start<'a>(input: &'a str, context: Context, scheme_type: SchemeType,
                            parse_error: ErrorHandler)
                            -> ParseResult<(Vec<String>, &'a str)> {
    let mut i = 0;
    // Relative path start state
    if !input.is_empty() {
        match input.char_at(0) {
            '/' => i = 1,
            '\\' => {
                try!(parse_error("Backslash"));
                i = 1;
            },
            _ => ()
        }
    }
    parse_path([], input.slice_from(i), context, scheme_type, parse_error)
}


fn parse_path<'a>(base_path: &[String], input: &'a str, context: Context,
                  scheme_type: SchemeType, parse_error: ErrorHandler)
                  -> ParseResult<(Vec<String>, &'a str)> {
    // Relative path state
    let mut path = base_path.to_owned();
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
                    try!(parse_error("Backslash"));
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '?' | '#' if context == UrlParserContext => {
                    end = i;
                    break
                },
                '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
                _ => {
                    try!(check_url_code_point(input, i, c, parse_error));
                    utf8_percent_encode(input.slice(i, next_i),
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
                if scheme_type == FileScheme
                   && path.is_empty()
                   && path_part.len() == 2
                   && starts_with_ascii_alpha(path_part.as_slice())
                   && path_part.as_slice().char_at(1) == '|' {
                    // Windows drive letter quirk
                    unsafe {
                        *path_part.as_mut_vec().get_mut(1) = b':'
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


fn parse_scheme_data<'a>(input: &'a str, parse_error: ErrorHandler)
                         -> ParseResult<(String, &'a str)> {
    let mut scheme_data = String::new();
    let mut end = input.len();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => {
                try!(check_url_code_point(input, i, c, parse_error));
                utf8_percent_encode(input.slice(i, next_i),
                                    SIMPLE_ENCODE_SET, &mut scheme_data);
            }
        }
    }
    Ok((scheme_data, input.slice_from(end)))
}


fn parse_query_and_fragment(input: &str, encoding_override: Option<EncodingRef>,
                            parse_error: ErrorHandler)
                            -> ParseResult<(Option<String>, Option<String>)> {
    if input.is_empty() {
        return Ok((None, None))
    }
    match input.char_at(0) {
        '#' => Ok((None, Some(try!(parse_fragment(input.slice_from(1), parse_error))))),
        '?' => {
            let (query, remaining) = try!(parse_query(
                input.slice_from(1), encoding_override, UrlParserContext, parse_error));
            let fragment = match remaining {
                Some(remaining) => Some(try!(parse_fragment(remaining, parse_error))),
                None => None
            };
            Ok((Some(query), fragment))
        },
        _ => fail!("Programming error. parse_query_and_fragment() should not \
                    have been called with input \"{}\"", input)
    }
}


pub fn parse_query<'a>(input: &'a str, encoding_override: Option<EncodingRef>, context: Context,
                   parse_error: ErrorHandler)
                   -> ParseResult<(String, Option<&'a str>)> {
    let mut query = String::new();
    let mut remaining = None;
    for (i, c) in input.char_indices() {
        match c {
            '#' if context == UrlParserContext => {
                remaining = Some(input.slice_from(i + 1));
                break
            },
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => {
                try!(check_url_code_point(input, i, c, parse_error));
                query.push_char(c);
            }
        }
    }
    let encoded;
    let query_bytes = match encoding_override {
        Some(encoding) => {
            encoded = encoding.encode(query.as_slice(), encoding::EncodeReplace).unwrap();
            encoded.as_slice()
        },
        None => query.as_bytes()  // UTF-8
    };
    let mut query_encoded = String::new();
    percent_encode(query_bytes.as_slice(), QUERY_ENCODE_SET, &mut query_encoded);
    Ok((query_encoded, remaining))
}


pub fn parse_fragment<'a>(input: &'a str, parse_error: ErrorHandler) -> ParseResult<String> {
    let mut fragment = String::new();
    for (i, c, next_i) in input.char_ranges() {
        match c {
            '\t' | '\n' | '\r' => try!(parse_error("Invalid character")),
            _ => {
                try!(check_url_code_point(input, i, c, parse_error));
                utf8_percent_encode(input.slice(i, next_i),
                                    SIMPLE_ENCODE_SET, &mut fragment);
            }
        }
    }
    Ok(fragment)
}


#[inline]
fn starts_with_ascii_alpha(string: &str) -> bool {
    match string.char_at(0) {
        'a'..'z' | 'A'..'Z' => true,
        _ => false,
    }
}

#[inline]
fn is_ascii_hex_digit(byte: u8) -> bool {
    match byte {
        b'a'..b'f' | b'A'..b'F' | b'0'..b'9' => true,
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
        'a'..'z' |
        'A'..'Z' |
        '0'..'9' |
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '-' |
        '.' | '/' | ':' | ';' | '=' | '?' | '@' | '_' | '~' |
        '\u00A0'..'\uD7FF' | '\uE000'..'\uFDCF' | '\uFDF0'..'\uFFFD' |
        '\U00010000'..'\U0001FFFD' | '\U00020000'..'\U0002FFFD' |
        '\U00030000'..'\U0003FFFD' | '\U00040000'..'\U0004FFFD' |
        '\U00050000'..'\U0005FFFD' | '\U00060000'..'\U0006FFFD' |
        '\U00070000'..'\U0007FFFD' | '\U00080000'..'\U0008FFFD' |
        '\U00090000'..'\U0009FFFD' | '\U000A0000'..'\U000AFFFD' |
        '\U000B0000'..'\U000BFFFD' | '\U000C0000'..'\U000CFFFD' |
        '\U000D0000'..'\U000DFFFD' | '\U000E1000'..'\U000EFFFD' |
        '\U000F0000'..'\U000FFFFD' | '\U00100000'..'\U0010FFFD' => true,
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


#[inline]
fn is_relative_scheme(scheme: &str) -> bool {
    is_match!(scheme, "ftp" | "file" | "gopher" | "http" | "https" | "ws" | "wss")
}


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
fn check_url_code_point(input: &str, i: uint, c: char, parse_error: ErrorHandler)
                        -> ParseResult<()> {
    if c == '%' {
        if !starts_with_2_hex(input.slice_from(i + 1)) {
            try!(parse_error("Invalid percent-encoded sequence"));
        }
    } else if !is_url_code_point(c) {
        try!(parse_error("Non-URL code point"));
    }
    Ok(())
}

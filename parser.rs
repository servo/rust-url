// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::ascii::StrAsciiExt;

use encoding;
use encoding::EncodingRef;
use encoding::all::UTF_8;

use super::{
    ParseResult, Url, RelativeSchemeData, OtherSchemeData,
    SchemeRelativeUrl, UserInfo, Host, Domain,
    utf8_percent_encode, percent_encode_byte,
    SimpleEncodeSet, DefaultEncodeSet, UserInfoEncodeSet};


macro_rules! is_match(
    ($value:expr, $($pattern:pat)|+) => (
        match $value { $($pattern)|+ => true, _ => false }
    );
)


macro_rules! ascii_nocheck(
    ($value: expr) => {
        unsafe { $value.to_ascii_nocheck() }
    }
)


fn parse_error(_message: &str) {
    // TODO
}


pub fn parse_url(input: &str, base_url: Option<&Url>) -> ParseResult<Url> {
    let input = input.trim_chars(&[' ', '\t', '\n', '\r', '\x0C']);
    let (scheme_result, remaining) = parse_scheme(input);
    match scheme_result {
        Some(scheme) => {
            if scheme.as_slice() == "file" {
                // Relative state?
                match base_url {
                    Some(base) if scheme == base.scheme => {
                        parse_error("Relative URL with a scheme");
                        parse_relative_url(scheme, remaining, base)
                    },
                    _ => parse_relative_url(scheme, remaining, &Url {
                        scheme: String::new(), query: None, fragment: None,
                        scheme_data: RelativeSchemeData(SchemeRelativeUrl {
                            userinfo: None, host: Domain(Vec::new()),
                            port: String::new(), path: Vec::new()
                        })
                    }),
                }
            } else if is_relative_scheme(scheme.as_slice()) {
                match base_url {
                    Some(base) if scheme == base.scheme => {
                        // Relative or authority state
                        if remaining.starts_with("//") {
                            parse_absolute_url(scheme, remaining)
                        } else {
                            parse_error("Relative URL with a scheme");
                            parse_relative_url(scheme, remaining, base)
                        }
                    },
                    _ => parse_absolute_url(scheme, remaining),
                }
            } else {
                // Scheme data state
                let (scheme_data, remaining) = parse_scheme_data(remaining);
                let (query, fragment) = parse_query_and_fragment(remaining);
                Ok(Url { scheme: scheme, scheme_data: OtherSchemeData(scheme_data),
                         query: query, fragment: fragment })
            }
        },
        // No-scheme state
        None => match base_url {
            None => Err("Relative URL without a base"),
            Some(base) => parse_relative_url(base.scheme.clone(), remaining, base)
        }
    }
}


fn parse_scheme<'a>(input: &'a str) -> (Option<String>, &'a str) {
    if input.is_empty() || !is_ascii_alpha(input.as_bytes()[0]) {
        return (None, input)
    }
    let mut i = 1;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            'a'..'z' | 'A'..'Z' | '0'..'9' | '+' | '-' | '.' => (),
            ':' => return (
                Some(input.slice_to(i).to_ascii_lower()),
                input.slice_from(i + 1),
            ),
            _ => return (None, input),
        }
        i += 1;
    }
    return (None, input)
}


fn parse_absolute_url<'a>(scheme: String, input: &'a str) -> ParseResult<Url> {
    // Authority first slash state
    let remaining = skip_slashes(input);
    // Authority state
    let (userinfo, remaining) = parse_userinfo(remaining);
    // Host state
    let (host, port, remaining) = match parse_hostname(remaining, scheme.as_slice()) {
        Err(message) => return Err(message),
        Ok(result) => result,
    };
    let (path, remaining) = parse_path_start(
        remaining,
        /* full_url= */ true,
        /* in_file_scheme= */ false);
    let scheme_data = RelativeSchemeData(SchemeRelativeUrl { userinfo: userinfo, host: host, port: port, path: path });
    let (query, fragment) = parse_query_and_fragment(remaining);
    Ok(Url { scheme: scheme, scheme_data: scheme_data, query: query, fragment: fragment })
}


fn parse_relative_url<'a>(scheme: String, input: &'a str, base: &Url) -> ParseResult<Url> {
    match base.scheme_data {
        OtherSchemeData(_) => Err("Relative URL with a non-relative-scheme base"),
        RelativeSchemeData(ref base_scheme_data) => if input.is_empty() {
            Ok(Url { scheme: scheme, scheme_data: base.scheme_data.clone(),
                     query: base.query.clone(), fragment: None })
        } else {
            let in_file_scheme = scheme.as_slice() == "file";
            match input.as_bytes()[0] as char {
                '/' | '\\' => {
                    // Relative slash state
                    if input.len() > 1 && is_match!(input.as_bytes()[1] as char, '/' | '\\') {
                        if in_file_scheme {
                            let remaining = input.slice_from(2);
                            let (host, remaining) = if remaining.len() >= 2
                               && is_ascii_alpha(remaining.as_bytes()[0])
                               && is_match!(remaining.as_bytes()[1] as char, ':' | '|')
                               && (remaining.len() == 2
                                   || is_match!(remaining.as_bytes()[2] as char,
                                                 '/' | '\\' | '?' | '#'))
                            {
                                // Windows drive letter quirk
                                (Domain(Vec::new()), remaining)
                            } else {
                                // File host state
                                match parse_file_host(remaining) {
                                    Err(message) => return Err(message),
                                    Ok(result) => result,
                                }
                            };
                            let (path, remaining) = parse_path_start(
                                remaining, /* full_url= */ true, in_file_scheme);
                            let scheme_data = RelativeSchemeData(SchemeRelativeUrl {
                                userinfo: None, host: host, port: String::new(), path: path });
                            let (query, fragment) = parse_query_and_fragment(remaining);
                            Ok(Url { scheme: scheme, scheme_data: scheme_data,
                                     query: query, fragment: fragment })
                        } else {
                            parse_absolute_url(scheme, input)
                        }
                    } else {
                        // Relative path state
                        let (path, remaining) = parse_path(
                            Vec::new(), input.slice_from(1), /* full_url= */ true, in_file_scheme);
                        let scheme_data = RelativeSchemeData(if in_file_scheme {
                            SchemeRelativeUrl {
                                userinfo: None, host: Domain(Vec::new()),
                                port: String::new(), path: path
                            }
                        } else {
                            SchemeRelativeUrl {
                                userinfo: base_scheme_data.userinfo.clone(),
                                host: base_scheme_data.host.clone(),
                                port: base_scheme_data.port.clone(),
                                path: path
                            }
                        });
                        let (query, fragment) = parse_query_and_fragment(remaining);
                        Ok(Url { scheme: scheme, scheme_data: scheme_data,
                                 query: query, fragment: fragment })
                    }
                },
                '?' => {
                    let (query, fragment) = parse_query_and_fragment(input);
                    Ok(Url { scheme: scheme, scheme_data: base.scheme_data.clone(),
                             query: query, fragment: fragment })
                },
                '#' => {
                    Ok(Url { scheme: scheme, scheme_data: base.scheme_data.clone(),
                             query: base.query.clone(),
                             fragment: Some(parse_fragment(input.slice_from(1))) })
                }
                _ => {
                    let (scheme_data, remaining) = if in_file_scheme
                       && input.len() >= 2
                       && is_ascii_alpha(input.as_bytes()[0])
                       && is_match!(input.as_bytes()[1] as char, ':' | '|')
                       && (input.len() == 2
                           || is_match!(input.as_bytes()[2] as char, '/' | '\\' | '?' | '#'))
                    {
                        // Windows drive letter quirk
                        let (path, remaining) = parse_path(
                            Vec::new(), input, /* full_url= */ true, in_file_scheme);
                         (RelativeSchemeData(SchemeRelativeUrl {
                            userinfo: None,
                            host: Domain(Vec::new()),
                            port: String::new(),
                            path: path
                        }), remaining)
                    } else {
                        let base_path = base_scheme_data.path.as_slice();
                        let initial_path = Vec::from_slice(
                            base_path.slice_to(base_path.len() - 1));
                        // Relative path state
                        let (path, remaining) = parse_path(
                            initial_path, input, /* full_url= */ true, in_file_scheme);
                        (RelativeSchemeData(SchemeRelativeUrl {
                            userinfo: base_scheme_data.userinfo.clone(),
                            host: base_scheme_data.host.clone(),
                            port: base_scheme_data.port.clone(),
                            path: path
                        }), remaining)
                    };
                    let (query, fragment) = parse_query_and_fragment(remaining);
                    Ok(Url { scheme: scheme, scheme_data: scheme_data,
                             query: query, fragment: fragment })
                }
            }
        }
    }

}


fn skip_slashes<'a>(input: &'a str) -> &'a str {
    let mut i = 0;
    let mut has_backslashes = false;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '/' => (),
            '\\' => has_backslashes = true,
            _ => break
        }
        i += 1;
    }
    if i != 2 || has_backslashes {
        parse_error("Expected two slashes")
    }
    input.slice_from(i)
}


fn parse_userinfo<'a>(input: &'a str) -> (Option<UserInfo>, &'a str) {
    let mut i = 0;
    let mut last_at = None;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '@' => last_at = Some(i),
            '/' | '\\' | '?' | '#' => break,
            _ => (),
        }
        i += 1;
    }
    match last_at {
        None => (None, input),
        Some(at) => (Some(parse_userinfo_inner(input.slice_to(at))),
                     input.slice_from(at + 1))
    }
}


fn parse_userinfo_inner<'a>(input: &'a str) -> UserInfo {
    let mut username = String::new();
    let mut i = 0;
    loop {
        if i >= input.len() {
            return UserInfo { username: username, password: None }
        }
        match input.as_bytes()[i] as char {
            ':' => {
                i += 1;
                break
            },
            '\t' | '\n' | '\r' => {
                parse_error("Invalid character");
                i += 1;
            },
            _ => {
                let range = input.char_range_at(i);
                if range.ch == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(range.ch) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, range.next), UserInfoEncodeSet, &mut username);
                i = range.next;
            }
        }
    }
    let mut password = String::new();
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '\t' | '\n' | '\r' => {
                parse_error("Invalid character");
                i += 1;
            },
            _ => {
                let range = input.char_range_at(i);
                if range.ch == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(range.ch) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, range.next), UserInfoEncodeSet, &mut password);
                i = range.next;
            }
        }
    }
    UserInfo { username: username, password: Some(password) }
}


fn parse_hostname<'a>(input: &'a str, scheme: &str) -> ParseResult<(Host, String, &'a str)> {
    let mut i = 0;
    let mut inside_square_brackets = false;
    let mut host_input = String::new();
    while i < input.len() {
        match input.as_bytes()[i] as char {
            ':' if !inside_square_brackets => return match Host::parse(host_input.as_slice()) {
                Err(message) => Err(message),
                Ok(host) => {
                    match parse_port(input.slice_from(i + 1), scheme) {
                        Err(message) => Err(message),
                        Ok((port, remaining)) => Ok((host, port, remaining)),
                    }
                }
            },
            '/' | '\\' | '?' | '#' => break,
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            c => {
                match c {
                    '[' => inside_square_brackets = true,
                    ']' => inside_square_brackets = false,
                    _ => (),
                }
                unsafe { host_input.push_byte(input.as_bytes()[i]) }
            }
        }
        i += 1;
    }
    match Host::parse(host_input.as_slice()) {
        Err(message) => Err(message),
        Ok(host) => Ok((host, String::new(), input.slice_from(i))),
    }
}


fn parse_port<'a>(input: &'a str, scheme: &str) -> ParseResult<(String, &'a str)> {
    let mut port = String::new();
    let mut has_initial_zero = false;
    let mut i = 0;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '1' .. '9' => unsafe { port.push_byte(input.as_bytes()[i]) },
            '0' => {
                if port.is_empty() {
                    has_initial_zero = true
                } else {
                    unsafe { port.push_byte(input.as_bytes()[i]) }
                }
            },
            '/' | '\\' | '?' | '#' => break,
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => return Err("Invalid port number")
        }
        i += 1;
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
    return Ok((port, input.slice_from(i)))
}


fn parse_file_host<'a>(input: &'a str) -> ParseResult<(Host, &'a str)> {
    let mut i = 0;
    let mut host_input = String::new();
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '/' | '\\' | '?' | '#' => break,
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => unsafe { host_input.push_byte(input.as_bytes()[i]) }
        }
        i += 1;
    }
    let host = if host_input.is_empty() {
        Domain(Vec::new())
    } else {
        match Host::parse(host_input.as_slice()) {
            Err(message) => return Err(message),
            Ok(host) => host,
        }
    };
    Ok((host, input.slice_from(i)))
}


fn parse_path_start<'a>(input: &'a str, full_url: bool, in_file_scheme: bool)
           -> (Vec<String>, &'a str) {
    let mut i = 0;
    // Relative path start state
    if !input.is_empty() {
        match input.as_bytes()[0] as char {
            '/' => i = 1,
            '\\' => {
                parse_error("Backslash");
                i = 1;
            },
            _ => ()
        }
    }
    parse_path(Vec::new(), input.slice_from(i), full_url, in_file_scheme)
}


fn parse_path<'a>(base_path: Vec<String>, input: &'a str, full_url: bool, in_file_scheme: bool)
           -> (Vec<String>, &'a str) {
    // Relative path state
    let mut path = base_path;
    let mut i = 0;
    loop {
        let mut path_part = String::new();
        let mut ends_with_slash = false;
        while i < input.len() {
            match input.as_bytes()[i] as char {
                '/' => {
                    i += 1;
                    ends_with_slash = true;
                    break
                },
                '\\' => {
                    parse_error("Backslash");
                    i += 1;
                    ends_with_slash = true;
                    break
                },
                '?' | '#' if full_url => break,
                '\t' | '\n' | '\r' => {
                    i += 1;
                    parse_error("Invalid character")
                },
                _ => {
                    let range = input.char_range_at(i);
                    if range.ch == '%' {
                        if !starts_with_2_hex(input.slice_from(i + 1)) {
                            parse_error("Invalid percent-encoded sequence")
                        }
                    } else if !is_url_code_point(range.ch) {
                        parse_error("Non-URL code point")
                    }

                    utf8_percent_encode(input.slice(i, range.next), DefaultEncodeSet, &mut path_part);
                    i = range.next;
                }
            }
        }
        let lower = path_part.as_slice().to_ascii_lower();
        match lower.as_slice() {
            ".." | ".%2e" | "%2e." | "%2e%2e" => {
                path.pop();
                if !ends_with_slash {
                    path.push(String::new());
                }
            },
            "." | "%2e" => {
                if !ends_with_slash {
                    path.push(String::new());
                }
            },
            _ => {
                if in_file_scheme
                   && path.is_empty()
                   && path_part.len() == 2
                   && is_ascii_alpha(path_part.as_bytes()[0])
                   && path_part.as_bytes()[1] == ('|' as u8) {
                    // Windows drive letter quirk
                    unsafe {
                        *path_part.as_mut_vec().get_mut(1) = ':' as u8
                    }
                }
                path.push(path_part)
            }
        }
        if !ends_with_slash {
            break
        }
    }
    (path, input.slice_from(i))
}


fn parse_scheme_data<'a>(input: &'a str) -> (String, &'a str) {
    let mut scheme_data = String::new();
    let mut i = 0;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '?' | '#' => break,
            '\t' | '\n' | '\r' => {
                parse_error("Invalid character");
                i += 1;
            },
            _ => {
                let range = input.char_range_at(i);
                if range.ch == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(range.ch) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, range.next), SimpleEncodeSet, &mut scheme_data);
                i = range.next;
            }
        }
    }
    (scheme_data, input.slice_from(i))
}


fn parse_query_and_fragment(input: &str) -> (Option<String>, Option<String>) {
    if input.is_empty() {
        (None, None)
    } else {
        match input.as_bytes()[0] as char {
            '#' => (None, Some(parse_fragment(input.slice_from(1)))),
            '?' => {
                let (query, remaining) = parse_query(
                    input.slice_from(1),
                    UTF_8 as EncodingRef,  // TODO
                    /* full_url = */ true);
                (Some(query), remaining.map(parse_fragment))
            },
            _ => fail!("Programming error")
        }
    }
}


fn parse_query<'a>(input: &'a str, encoding_override: EncodingRef, full_url: bool)
               -> (String, Option<&'a str>) {
    let mut query = String::new();
    let mut i = 0;
    let mut remaining = None;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '#' if full_url => {
                remaining = Some(input.slice_from(i + 1));
                break
            },
            '\t' | '\n' | '\r' => {
                parse_error("Invalid character");
                i += 1;
            },
            _ => {
                let range = input.char_range_at(i);
                if range.ch == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(range.ch) {
                    parse_error("Non-URL code point")
                }

                query.push_char(range.ch);
                i = range.next;
            }
        }
    }
    let query_bytes = encoding_override.encode(query.as_slice(), encoding::EncodeReplace).unwrap();
    let mut query_encoded = String::new();
    for &byte in query_bytes.iter() {
        match byte {
            0x00 .. 0x20 | 0x22 | 0x23 | 0x3C | 0x3E | 0x60 | 0x7E .. 0xFF
            => percent_encode_byte(byte, &mut query_encoded),
            _
            => unsafe { query_encoded.push_byte(byte) }
        }
    }
    (query_encoded, remaining)
}


fn parse_fragment<'a>(input: &'a str) -> String {
    let mut fragment = String::new();
    let mut i = 0;
    while i < input.len() {
        match input.as_bytes()[i] as char {
            '\t' | '\n' | '\r' => {
                parse_error("Invalid character");
                i += 1;
            },
            _ => {
                let range = input.char_range_at(i);
                if range.ch == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(range.ch) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, range.next), SimpleEncodeSet, &mut fragment);
                i = range.next;
            }
        }
    }
    fragment
}


#[inline]
fn is_ascii_alpha(byte: u8) -> bool {
    match byte as char {
        'a'..'z' | 'A'..'Z' => true,
        _ => false,
    }
}

#[inline]
fn is_ascii_hex_digit(byte: u8) -> bool {
    match byte as char {
        'a'..'f' | 'A'..'F' | '0'..'9' => true,
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
        '\u00A0'..'\uD7FF' | '\uE000'..'\uFDCF' | '\uFDF0'..'\uFFEF' |
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
// U+FFF0 to U+FFFF
// Last two of each plane: U+__FFFE to U+__FFFF for __ in 01 to 10 hex


fn is_relative_scheme(scheme: &str) -> bool {
    is_match!(scheme, "ftp" | "file" | "gopher" | "http" | "https" | "ws" | "wss")
}

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


fn parse_error(_message: &str) {
    // TODO
}


pub fn parse_url(input: &str, base_url: Option<&Url>) -> ParseResult<Url> {
    let input = input.trim_chars(&[' ', '\t', '\n', '\r', '\x0C']);
    match parse_scheme(input) {
        Some((scheme, remaining)) => {
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
            Some(base) => parse_relative_url(base.scheme.clone(), input, base)
        }
    }
}


fn parse_scheme<'a>(input: &'a str) -> Option<(String, &'a str)> {
    if !input.is_empty() && starts_with_ascii_alpha(input) {
        for (i, c) in input.char_indices() {
            match c {
                'a'..'z' | 'A'..'Z' | '0'..'9' | '+' | '-' | '.' => (),
                ':' => return Some((
                    input.slice_to(i).to_ascii_lower(),
                    input.slice_from(i + 1),
                )),
                _ => break,
            }
        }
    }
    None
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
            match input.char_at(0) {
                '/' | '\\' => {
                    // Relative slash state
                    if input.len() > 1 && is_match!(input.char_at(1), '/' | '\\') {
                        if in_file_scheme {
                            let remaining = input.slice_from(2);
                            let (host, remaining) = if remaining.len() >= 2
                               && starts_with_ascii_alpha(remaining)
                               && is_match!(remaining.char_at(1), ':' | '|')
                               && (remaining.len() == 2
                                   || is_match!(remaining.char_at(2),
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
                       && starts_with_ascii_alpha(input)
                       && is_match!(input.char_at(1), ':' | '|')
                       && (input.len() == 2
                           || is_match!(input.char_at(2), '/' | '\\' | '?' | '#'))
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
    let first_non_slash = input.find(|c| !is_match!(c, '/' | '\\')).unwrap_or(input.len());
    if input.slice_to(first_non_slash) != "//" {
        parse_error("Expected two slashes")
    }
    input.slice_from(first_non_slash)
}


fn parse_userinfo<'a>(input: &'a str) -> (Option<UserInfo>, &'a str) {
    let mut last_at = None;
    for (i, c) in input.char_indices() {
        match c {
            '@' => last_at = Some(i),
            '/' | '\\' | '?' | '#' => break,
            _ => (),
        }
    }
    match last_at {
        None => (None, input),
        Some(at) => (Some(parse_userinfo_inner(input.slice_to(at))),
                     input.slice_from(at + 1))
    }
}


fn parse_userinfo_inner(input: &str) -> UserInfo {
    let mut username = String::new();
    for (i, c) in input.char_indices() {
        match c {
            ':' => return parse_userinfo_password(input.slice_from(i + 1), username),
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => {
                if c == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(c) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, i + c.len_utf8_bytes()),
                                    UserInfoEncodeSet, &mut username);
            }
        }
    }
    UserInfo { username: username, password: None }
}


fn parse_userinfo_password(input: &str, username: String) -> UserInfo {
    let mut password = String::new();
    for (i, c) in input.char_indices() {
        match c {
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => {
                if c == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(c) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, i + c.len_utf8_bytes()),
                                    UserInfoEncodeSet, &mut password);
            }
        }
    }
    UserInfo { username: username, password: Some(password) }
}


fn parse_hostname<'a>(input: &'a str, scheme: &str) -> ParseResult<(Host, String, &'a str)> {
    let mut inside_square_brackets = false;
    let mut host_input = String::new();
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            ':' if !inside_square_brackets => return match Host::parse(host_input.as_slice()) {
                Err(message) => Err(message),
                Ok(host) => {
                    match parse_port(input.slice_from(i + 1), scheme) {
                        Err(message) => Err(message),
                        Ok((port, remaining)) => Ok((host, port, remaining)),
                    }
                }
            },
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
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
    match Host::parse(host_input.as_slice()) {
        Err(message) => Err(message),
        Ok(host) => Ok((host, String::new(), input.slice_from(end))),
    }
}


fn parse_port<'a>(input: &'a str, scheme: &str) -> ParseResult<(String, &'a str)> {
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
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => return Err("Invalid port number")
        }
    }
    if port.is_empty() && has_initial_zero {
        port.push_str("0")
    }
    match (scheme, port.as_slice()) {
        ("ftp", "21") | ("gopher", "70") | ("http", "80") |
        ("https", "443") | ("ws", "80") | ("wss", "443")
        => port = String::new(),
        _ => (),
    }
    return Ok((port, input.slice_from(end)))
}


fn parse_file_host<'a>(input: &'a str) -> ParseResult<(Host, &'a str)> {
    let mut host_input = String::new();
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            '/' | '\\' | '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => host_input.push_char(c)
        }
    }
    let host = if host_input.is_empty() {
        Domain(Vec::new())
    } else {
        match Host::parse(host_input.as_slice()) {
            Err(message) => return Err(message),
            Ok(host) => host,
        }
    };
    Ok((host, input.slice_from(end)))
}


fn parse_path_start<'a>(input: &'a str, full_url: bool, in_file_scheme: bool)
           -> (Vec<String>, &'a str) {
    let mut i = 0;
    // Relative path start state
    if !input.is_empty() {
        match input.char_at(0) {
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
    let mut iter = input.char_indices();
    let mut end;
    loop {
        let mut path_part = String::new();
        let mut ends_with_slash = false;
        end = input.len();
        for (i, c) in iter {
            match c {
                '/' => {
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '\\' => {
                    parse_error("Backslash");
                    ends_with_slash = true;
                    end = i;
                    break
                },
                '?' | '#' if full_url => {
                    end = i;
                    break
                },
                '\t' | '\n' | '\r' => {
                    parse_error("Invalid character")
                },
                _ => {
                    if c == '%' {
                        if !starts_with_2_hex(input.slice_from(i + 1)) {
                            parse_error("Invalid percent-encoded sequence")
                        }
                    } else if !is_url_code_point(c) {
                        parse_error("Non-URL code point")
                    }

                    utf8_percent_encode(input.slice(i, i + c.len_utf8_bytes()),
                                        DefaultEncodeSet, &mut path_part);
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
                if in_file_scheme
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
    (path, input.slice_from(end))
}


fn parse_scheme_data<'a>(input: &'a str) -> (String, &'a str) {
    let mut scheme_data = String::new();
    let mut end = input.len();
    for (i, c) in input.char_indices() {
        match c {
            '?' | '#' => {
                end = i;
                break
            },
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => {
                if c == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(c) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, i + c.len_utf8_bytes()),
                                    SimpleEncodeSet, &mut scheme_data);
            }
        }
    }
    (scheme_data, input.slice_from(end))
}


fn parse_query_and_fragment(input: &str) -> (Option<String>, Option<String>) {
    if input.is_empty() {
        (None, None)
    } else {
        match input.char_at(0) {
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
    let mut remaining = None;
    for (i, c) in input.char_indices() {
        match c {
            '#' if full_url => {
                remaining = Some(input.slice_from(i + 1));
                break
            },
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => {
                if c == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(c) {
                    parse_error("Non-URL code point")
                }
                query.push_char(c);
            }
        }
    }
    let query_bytes = encoding_override.encode(query.as_slice(), encoding::EncodeReplace).unwrap();
    let mut query_encoded = String::new();
    for &byte in query_bytes.iter() {
        match byte {
            b'\x00'.. b' ' | b'"' | b'#' | b'<' | b'>' | b'`' | b'~'..b'\xFF'
            => percent_encode_byte(byte, &mut query_encoded),
            _
            => unsafe { query_encoded.push_byte(byte) }
        }
    }
    (query_encoded, remaining)
}


fn parse_fragment<'a>(input: &'a str) -> String {
    let mut fragment = String::new();
    for (i, c) in input.char_indices() {
        match c {
            '\t' | '\n' | '\r' => parse_error("Invalid character"),
            _ => {
                if c == '%' {
                    if !starts_with_2_hex(input.slice_from(i + 1)) {
                        parse_error("Invalid percent-encoded sequence")
                    }
                } else if !is_url_code_point(c) {
                    parse_error("Non-URL code point")
                }

                utf8_percent_encode(input.slice(i, i + c.len_utf8_bytes()),
                                    SimpleEncodeSet, &mut fragment);
            }
        }
    }
    fragment
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

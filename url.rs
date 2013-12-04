// Copyright 2013 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[link(name = "url", vers = "0.1")];
#[crate_type = "lib"];
#[feature(globs, macro_rules)];

extern mod encoding;

use encoding::Encoding;
use encoding::all::UTF_8;
use encoding::label::encoding_from_whatwg_label;


pub struct URL {
    scheme: ~str,
    scheme_data: SchemeData,
    query: Option<~str>,  // parse_form_urlencoded() parses this into ~[(~str, ~str)]
    fragment: Option<~str>,
}

pub enum SchemeData {
    RelativeSchemeData(SchemeRelativeURL),
    OtherSchemeData(~str)
}

pub struct SchemeRelativeURL {
    userinfo: Option<UserInfo>,
    host: Host,
    port: ~str,
    path: ~[~str],
}

pub struct UserInfo {
    username: ~str,
    password: Option<~str>,
}

pub enum Host {
    Domain(~[~str]),
    IPv6(IPv6Address)
}


pub struct IPv6Address {
    pieces: [u16, ..8]
}


pub type ParseResult<T> = Result<T, &'static str>;


impl URL {
    pub fn parse(input: &str, base_url: Option<URL>) -> Option<URL> {
        let _ = input;
        let _ = base_url;
        // TODO
        None
    }

    pub fn serialize(&self) -> ~str {
        let mut result = self.serialize_no_fragment();
        match self.fragment {
            None => (),
            Some(ref fragment) => {
                result.push_str("#");
                result.push_str(fragment.as_slice());
            }
        }
        result
    }

    pub fn serialize_no_fragment(&self) -> ~str {
        let mut result = self.scheme.to_owned();
        result.push_str(":");
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeURL {
                userinfo: ref userinfo, host: ref host, port: ref port, path: ref path
            }) => {
                result.push_str("//");
                match userinfo {
                    &None => (),
                    &Some(UserInfo { username: ref username, password: ref password })
                    => if username.len() > 0 || password.is_some() {
                        result.push_str(username.as_slice());
                        match password {
                            &None => (),
                            &Some(ref password) => {
                                result.push_str(":");
                                result.push_str(password.as_slice());
                            }
                        }
                        result.push_str("@");
                    }
                }
                result.push_str(host.serialize());
                if port.len() > 0 {
                    result.push_str(":");
                    result.push_str(port.as_slice());
                }
                if path.len() > 0 {
                    for path_part in path.iter() {
                        result.push_str("/");
                        result.push_str(path_part.as_slice());
                    }
                } else {
                    result.push_str("/");
                }
            },
            OtherSchemeData(ref data) => result.push_str(data.as_slice()),
        }
        match self.query {
            None => (),
            Some(ref query) => {
                result.push_str("?");
                result.push_str(query.as_slice());
            }
        }
        result
    }
}


impl Host {
    pub fn parse(input: &str) -> ParseResult<Host> {
        if input.len() == 0 {
            Err("Empty host")
        } else if input[0] == '[' as u8 {
            if input[input.len()] == ']' as u8 {
                match IPv6Address::parse(input.slice(1, input.len() - 1)) {
                    Some(address) => Ok(IPv6(address)),
                    None => Err("Invalid IPv6 address"),
                }
            } else {
                Err("Invalid IPv6 address")
            }
        } else {
            let mut percent_encoded = ~"";
            utf8_percent_encode(input, SimpleEncodeSet, &mut percent_encoded);
            let bytes = percent_decode(percent_encoded);
            let decoded = UTF_8.decode(bytes, encoding::DecodeReplace).unwrap();
            let mut labels = ~[];
            for label in decoded.split_iter(&['.', '\u3002', '\uFF0E', '\uFF61']) {
                // TODO: Remove this check and use IDNA "domain to ASCII"
                // TODO: switch to .map(domain_label_to_ascii).collect() then.
                if label.as_bytes().iter().any(|b| *b >= 0x80) {
                    return Err("Non-ASCII domains (IDNA) are not supported yet.")
                }
                labels.push(label.to_owned())
            }
            Ok(Domain(labels))
        }
    }

    pub fn serialize(&self) -> ~str {
        match *self {
            Domain(ref labels) => labels.connect("."),
            IPv6(ref address) => format!("[{}]", address.serialize()),
        }
    }
}


macro_rules! matches(
    ($value: expr, ($pattern: pat)|+) => {
        match $value {
            $($pattern)|+ => true,
            _ => false,
        }
    };
)


impl IPv6Address {
    pub fn parse(input: &str) -> Option<IPv6Address> {
        let len = input.len();
        let mut is_ip_v4 = false;
        let mut pieces = [0, 0, 0, 0, 0, 0, 0, 0];
        let mut piece_pointer = 0u;
        let mut compress_pointer = None;
        let mut i = 0u;
        if input[0] == ':' as u8 {
            if input[1] != ':' as u8 {
                return None
            }
            i = 2;
            piece_pointer = 1;
            compress_pointer = Some(1u);
        }

        while i < len {
            if piece_pointer == 8 {
                return None
            }
            if input[i] == ':' as u8 {
                if compress_pointer.is_some() {
                    return None
                }
                piece_pointer += 1;
                compress_pointer = Some(piece_pointer);
                continue
            }
            let start = i;
            let end = len.min(&(start + 4));
            let mut value = 0u16;
            while i < end {
                match byte_to_hex(input[i]) {
                    Some(digit) => {
                        value = value * 0x10 + digit as u16;
                        i += 1;
                    },
                    None => {
                        if input[i] == 0x2E {  // .
                            if i == start {
                                return None
                            }
                            i = start;
                            is_ip_v4 = true;
                            break
                        }
                        if input[i]  == 0x3A { // :
                            i += 1;
                            if i == len {
                                return None
                            }
                            break
                        }
                        return None
                    }
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
                return None
            }
            let mut dots_seen = 0u;
            while i < len {
                let mut value = 0u16;
                while i < len {
                    let digit = match input[i] {
                        c @ 0x30 .. 0x39 => c - 0x30,  // 0..9
                        _ => break
                    };
                    value = value * 10 + digit as u16;
                    if value > 255 {
                        return None
                    }
                }
                if dots_seen < 3 && !(i < len && input[i] == '.' as u8) {
                    return None
                }
                pieces[piece_pointer] = pieces[piece_pointer] * 0x100 + value;
                if dots_seen == 0 || dots_seen == 2 {
                    piece_pointer += 1;
                }
                i += 1;
                if dots_seen == 3 && i < len {
                    return None
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
                return None
            }
        }
        Some(IPv6Address { pieces: pieces })
    }

    pub fn serialize(&self) -> ~str {
        let mut output = ~"";
        let (compress_start, compress_end) = longest_zero_sequence(&self.pieces);
        let mut i = 0;
        while i < 8 {
            if i == compress_start {
                output.push_str(if i == 0 { "::" } else { ":" });
                if compress_end < 8 {
                    i = compress_end;
                } else {
                    break;
                }
            }
            output.push_str(self.pieces[i].to_str_radix(16));
            if i < 7 {
                output.push_str(":");
            }
        }
        output
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
        if pieces[i] == 0 {
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
fn byte_to_hex(byte: u8) -> Option<u8> {
    match byte {
        0x30 .. 0x39 => Some(byte - 0x30),  // 0..9
        0x41 .. 0x46 => Some(byte + 10 - 0x41),  // A..F
        0x61 .. 0x66 => Some(byte + 10 - 0x61),  // a..f
        _ => None
    }
}


enum EncodeSet {
    SimpleEncodeSet,
    DefaultEncodeSet,
    PasswordEncodeSet,
    UsernameEncodeSet
}


#[inline]
fn utf8_percent_encode(input: &str, encode_set: EncodeSet, output: &mut ~str) {
    use Default = self::DefaultEncodeSet;
    use Password = self::PasswordEncodeSet;
    use Username = self::UsernameEncodeSet;
    for &byte in input.as_bytes().iter() {
        if byte < 0x20 || byte > 0x7E || match byte as char {
            ' ' | '"' | '#' | '<' | '>' | '?' | '`'
            => match encode_set { Default | Password | Username => true, _ => false },
            '/' | '@' | '\\'
            => match encode_set { Password | Username => true, _ => false },
            ':'
            => match encode_set { Username => true, _ => false },
            _ => false,
        } {
            output.push_str(percent_encode_byte(byte))
        } else {
            use std::str::raw::push_byte;
            unsafe { push_byte(output, byte) }
        }
    }
}


#[inline]
fn percent_encode_byte(byte: u8) -> ~str {
    format!("%{:02X}", byte)
}


/// Fails on non-ASCII input
#[inline]
fn percent_decode(input: &str) -> ~[u8] {
    let mut output = ~[];
    let mut i = 0u;
    while i < input.len() {
        let c = input[i];
        if c == '%' as u8 && i + 2 < input.len() {
            match (byte_to_hex(input[i + 1]), byte_to_hex(input[i + 2])) {
                (Some(h), Some(l)) => {
                    output.push(h * 0x10 + l);
                    i += 3;
                    continue
                },
                _ => (),
            }
        }

        assert!(c < 0x80);
        output.push(c);
        i += 1;
    }
    output
}


pub fn parse_form_urlencoded(input: &str,
                             encoding_override: Option<&'static Encoding>,
                             use_charset: bool,
                             mut isindex: bool)
                          -> ~[(~str, ~str)] {
    let mut encoding_override = match encoding_override {
        Some(encoding) => encoding,
        None => UTF_8 as &'static Encoding,
    };
    let mut pairs = ~[];
    for string in input.split_iter('&') {
        if string.len() > 0 {
            let (name, value) = match string.find('=') {
                Some(position) => (string.slice_to(position), string.slice_from(position + 1)),
                None => if isindex { ("", string) } else { (string, "") }
            };
            let name = name.replace("+", " ");
            let value = value.replace("+", " ");
            if use_charset && name.as_slice() == "_charset_" {
                match encoding_from_whatwg_label(value) {
                    Some(encoding) => encoding_override = encoding,
                    None => (),
                }
            }
            pairs.push((name, value));
        }
        isindex = false;
    }

    #[inline]
    fn decode(input: &~str, encoding_override: &'static Encoding) -> ~str {
        let bytes = percent_decode(input.as_slice());
        encoding_override.decode(bytes, encoding::DecodeReplace).unwrap()
    }

    for pair in pairs.mut_iter() {
        let new_pair = {
            let &(ref name, ref value) = pair;
            (decode(name, encoding_override), decode(value, encoding_override))
        };
        *pair = new_pair;
    }
    pairs
}


pub fn serialize_form_urlencoded(pairs: ~[(~str, ~str)],
                                 encoding_override: Option<&'static Encoding>) {

    #[inline]
    fn byte_serialize(input: &str, output: &mut ~str,
                     encoding_override: Option<&'static Encoding>) {
        use std::cast::transmute;

        let keep_alive;
        let input = match encoding_override {
            None => input.as_bytes(),  // "Encode" to UTF-8
            Some(encoding) => {
                keep_alive = encoding.encode(input, encoding::EncodeNcrEscape).unwrap();
                keep_alive.as_slice()
            }
        };

        for byte in input.iter() {
            match *byte {
                0x20 => output.push_str("+"),
                0x2A | 0x2D | 0x2E | 0x30 .. 0x39 | 0x41 .. 0x5A | 0x5F | 0x61 .. 0x7A
                => output.push_str(unsafe { transmute(&[*byte]) }),
                _ => output.push_str(percent_encode_byte(*byte)),
            }
        }
    }

    let mut output = ~"";
    for &(ref name, ref value) in pairs.iter() {
        // TODO: add an encoding_override parameter and support other encodings.
        if output.len() > 0 {
            output.push_str("&");
            byte_serialize(name.as_slice(), &mut output, encoding_override);
            output.push_str("=");
            byte_serialize(value.as_slice(), &mut output, encoding_override);
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{char, u32};
    use super::*;

    #[test]
    fn test() {
        for test in parse_test_data(include_str!("urltestdata.txt")).move_iter() {
            let Test {
                input: input,
                base: base,
                scheme: expected_scheme,
                username: expected_username,
                password: expected_password,
                host: expected_host,
                port: expected_port,
                path: expected_path,
                query: expected_query,
                fragment: expected_fragment
            } = test;
            let base = URL::parse(base, None).unwrap();
            let url = URL::parse(input, Some(base));
            if expected_scheme.is_none() {
                assert!(url.is_none(), "Expected a parse error");
                continue
            }
            let URL {
                scheme: scheme,
                scheme_data: scheme_data,
                query: query,
                fragment: fragment
            } = url.unwrap();

            assert_eq!(Some(scheme), expected_scheme);
            match scheme_data {
                RelativeSchemeData(SchemeRelativeURL {
                    userinfo: userinfo, host: host, port: port, path: path
                }) => {
                    let (username, password) = match userinfo {
                        Some(UserInfo { username: username, password: password })
                        => (Some(username), password),
                        _ => (None, None),
                    };
                    assert_eq!(username, expected_username);
                    assert_eq!(password, expected_password);
                    assert_eq!(Some(host.serialize()), expected_host)
                    assert_eq!(Some(port), expected_port);
                    assert_eq!(Some(path.connect("/")), expected_path);
                },
                OtherSchemeData(scheme_data) => {
                    assert_eq!(Some(scheme_data), expected_path);
                    assert_eq!(None, expected_username);
                    assert_eq!(None, expected_password);
                    assert_eq!(None, expected_host);
                    assert_eq!(None, expected_port);
                },
            }
            assert_eq!(query, expected_query);
            assert_eq!(fragment, expected_fragment);
        }
    }

    struct Test {
        input: ~str,
        base: ~str,
        scheme: Option<~str>,
        username: Option<~str>,
        password: Option<~str>,
        host: Option<~str>,
        port: Option<~str>,
        path: Option<~str>,
        query: Option<~str>,
        fragment: Option<~str>,
    }

    fn parse_test_data(input: &str) -> ~[Test] {
        let mut tests: ~[Test] = ~[];
        for line in input.line_iter() {
            if line == "" || line[0] == ('#' as u8) {
                continue
            }
            let mut pieces = line.split_iter(' ').to_owned_vec();
            let input = unescape(pieces.shift());
            let mut test = Test {
                input: input,
                base: if pieces.is_empty() {
                    tests[tests.len() - 1].base.to_owned()
                } else {
                    unescape(pieces.shift())
                },
                scheme: None,
                username: None,
                password: None,
                host: None,
                port: None,
                path: None,
                query: None,
                fragment: None,
            };
            for piece in pieces.move_iter() {
                if piece != "" || piece[0] == ('#' as u8) {
                    continue
                }
                let colon = piece.find(':').unwrap();
                let value = piece.slice_from(colon + 1).to_owned();
                match piece.slice_to(colon) {
                    "s" => test.scheme = Some(value),
                    "u" => test.username = Some(value),
                    "pass" => test.password = Some(value),
                    "h" => test.host = Some(value),
                    "p" => test.path = Some(value),
                    "q" => test.query = Some(value),
                    "f" => test.fragment = Some(value),
                    _ => fail!("Invalid token")
                }
            }
            tests.push(test)
        }
        tests
    }

    fn unescape(input: &str) -> ~str {
        let mut output = ~"";
        let mut chars = input.iter();
        loop {
            match chars.next() {
                None => return output,
                Some(c) => output.push_char(
                    if c == '\\' {
                        match chars.next().unwrap() {
                            '\\' => '\\',
                            'n' => '\n',
                            'r' => '\r',
                            's' => ' ',
                            't' => '\t',
                            'f' => '\x0C',
                            'u' => {
                                let mut hex = ~"";
                                hex.push_char(chars.next().unwrap());
                                hex.push_char(chars.next().unwrap());
                                hex.push_char(chars.next().unwrap());
                                hex.push_char(chars.next().unwrap());
                                u32::parse_bytes(hex.as_bytes(), 16)
                                    .and_then(char::from_u32).unwrap()
                            }
                            _ => fail!("Invalid test data input"),
                        }
                    } else {
                        c
                    }
                )
            }
        }
    }
}

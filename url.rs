// Copyright 2013 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[link(name = "url", vers = "0.1")];
#[crate_type = "lib"];
#[feature(macro_rules)];

extern mod encoding;

#[cfg(test)]
extern mod extra;

use std::ascii::Ascii;

use encoding::EncodingRef;
use encoding::Encoding;
use encoding::all::UTF_8;
use encoding::label::encoding_from_whatwg_label;


pub mod punycode;

#[cfg(test)]
mod tests;


pub struct URL {
    scheme: ~[Ascii],
    scheme_data: SchemeData,
    query: Option<~[Ascii]>,  // parse_form_urlencoded() parses this into ~[(~str, ~str)]
    fragment: Option<~[Ascii]>,
}

pub enum SchemeData {
    RelativeSchemeData(SchemeRelativeURL),
    OtherSchemeData(~[Ascii])
}

pub struct SchemeRelativeURL {
    userinfo: Option<UserInfo>,
    host: Host,
    port: ~[Ascii],
    path: ~[~[Ascii]],
}

pub struct UserInfo {
    username: ~[Ascii],
    password: Option<~[Ascii]>,
}

pub enum Host {
    Domain(~[~[Ascii]]),
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

    pub fn serialize(&self) -> ~[Ascii] {
        let mut result = self.serialize_no_fragment();
        match self.fragment {
            None => (),
            Some(ref fragment) => {
                result.push('#'.to_ascii());
                result.push_all(fragment.as_slice());
            }
        }
        result
    }

    pub fn serialize_no_fragment(&self) -> ~[Ascii] {
        let mut result = self.scheme.to_owned();
        result.push(':'.to_ascii());
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeURL {
                userinfo: ref userinfo, host: ref host, port: ref port, path: ref path
            }) => {
                result.push_all("//".to_ascii());
                match userinfo {
                    &None => (),
                    &Some(UserInfo { username: ref username, password: ref password })
                    => if username.len() > 0 || password.is_some() {
                        result.push_all(username.as_slice());
                        match password {
                            &None => (),
                            &Some(ref password) => {
                                result.push(':'.to_ascii());
                                result.push_all(password.as_slice());
                            }
                        }
                        result.push('@'.to_ascii());
                    }
                }
                result.push_all(host.serialize());
                if port.len() > 0 {
                    result.push(':'.to_ascii());
                    result.push_all(port.as_slice());
                }
                if path.len() > 0 {
                    for path_part in path.iter() {
                        result.push('/'.to_ascii());
                        result.push_all(path_part.as_slice());
                    }
                } else {
                    result.push('/'.to_ascii());
                }
            },
            OtherSchemeData(ref data) => result.push_all(data.as_slice()),
        }
        match self.query {
            None => (),
            Some(ref query) => {
                result.push('?'.to_ascii());
                result.push_all(query.as_slice());
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
            let mut percent_encoded = ~[];
            utf8_percent_encode(input, SimpleEncodeSet, &mut percent_encoded);
            let bytes = percent_decode(percent_encoded);
            let decoded = UTF_8.decode(bytes, encoding::DecodeReplace).unwrap();
            let mut labels = ~[];
            for label in decoded.split_iter(&['.', '\u3002', '\uFF0E', '\uFF61']) {
                // TODO: Remove this check and use IDNA "domain to ASCII"
                // TODO: switch to .map(domain_label_to_ascii).collect() then.
                if label.is_ascii() {
                    labels.push(unsafe { label.to_ascii_nocheck() }.to_owned())
                } else {
                    return Err("Non-ASCII domains (IDNA) are not supported yet.")
                }
            }
            Ok(Domain(labels))
        }
    }

    pub fn serialize(&self) -> ~[Ascii] {
        match *self {
            Domain(ref labels) => labels.connect_vec(&'.'.to_ascii()),
            IPv6(ref address) => {
                let mut result = ~['['.to_ascii()];
                result.push_all(address.serialize());
                result.push(']'.to_ascii());
                result
            }
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
                match from_hex(input[i]) {
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

    pub fn serialize(&self) -> ~[Ascii] {
        let mut output = ~[];
        let (compress_start, compress_end) = longest_zero_sequence(&self.pieces);
        let mut i = 0;
        while i < 8 {
            if i == compress_start {
                output.push(':'.to_ascii());
                if i == 0 {
                    output.push(':'.to_ascii());
                }
                if compress_end < 8 {
                    i = compress_end;
                } else {
                    break;
                }
            }
            let hex = self.pieces[i].to_str_radix(16);
            // No need to check that hex digits are ASCII
            output.push_all(unsafe { hex.to_ascii_nocheck() });
            if i < 7 {
                output.push(':'.to_ascii());
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
fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        0x30 .. 0x39 => Some(byte - 0x30),  // 0..9
        0x41 .. 0x46 => Some(byte + 10 - 0x41),  // A..F
        0x61 .. 0x66 => Some(byte + 10 - 0x61),  // a..f
        _ => None
    }
}

#[inline]
fn to_hex_upper(value: u8) -> Ascii {
    let digit = match value {
        0 .. 9 => value + 0x30,
        10 .. 15 => value - 10 + 0x41,
        _ => fail!()
    };
    unsafe { digit.to_ascii_nocheck() }
}


enum EncodeSet {
    SimpleEncodeSet,
    DefaultEncodeSet,
    PasswordEncodeSet,
    UsernameEncodeSet
}


#[inline]
fn utf8_percent_encode(input: &str, encode_set: EncodeSet, output: &mut ~[Ascii]) {
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
            percent_encode_byte(byte, output)
        } else {
            output.push(unsafe { byte.to_ascii_nocheck() })  // Already checked
        }
    }
}


#[inline]
fn percent_encode_byte(byte: u8, output: &mut ~[Ascii]) {
    output.push_all(['%'.to_ascii(), to_hex_upper(byte >> 4), to_hex_upper(byte & 0x0F)])
}


#[inline]
fn percent_decode(input: &[Ascii]) -> ~[u8] {
    let mut output = ~[];
    let mut i = 0u;
    while i < input.len() {
        let c = input[i];
        if c == '%'.to_ascii() && i + 2 < input.len() {
            match (from_hex(input[i + 1].to_byte()),
                   from_hex(input[i + 2].to_byte())) {
                (Some(h), Some(l)) => {
                    output.push(h * 0x10 + l);
                    i += 3;
                    continue
                },
                _ => (),
            }
        }

        output.push(c.to_byte());
        i += 1;
    }
    output
}


pub fn parse_form_urlencoded(input: &[Ascii],
                             encoding_override: Option<EncodingRef>,
                             use_charset: bool,
                             mut isindex: bool)
                          -> ~[(~str, ~str)] {
    let mut encoding_override = encoding_override.unwrap_or(UTF_8 as EncodingRef);
    let mut pairs = ~[];
    for string in input.split_iter(|&c| c == '&'.to_ascii()) {
        if string.len() > 0 {
            let (name, value) = match string.position_elem(&'='.to_ascii()) {
                Some(position) => (string.slice_to(position), string.slice_from(position + 1)),
                None => if isindex { (&[], string) } else { (string, &[]) }
            };
            let name = name.to_str_ascii().replace("+", " ");
            let value = value.to_str_ascii().replace("+", " ");
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
    fn decode(input: &~str, encoding_override: EncodingRef) -> ~str {
        // No need to check as input comes from &[Ascii].to_str_ascii().replace("+", " ")
        let bytes = percent_decode(unsafe { input.as_slice().to_ascii_nocheck() });
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
                                 encoding_override: Option<EncodingRef>)
                              -> ~[Ascii] {
    #[inline]
    fn byte_serialize(input: &str, output: &mut ~[Ascii],
                     encoding_override: Option<EncodingRef>) {
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
                0x20 => output.push('+'.to_ascii()),
                0x2A | 0x2D | 0x2E | 0x30 .. 0x39 | 0x41 .. 0x5A | 0x5F | 0x61 .. 0x7A
                => output.push(unsafe { byte.to_ascii_nocheck() }),
                _ => percent_encode_byte(*byte, output),
            }
        }
    }

    let mut output = ~[];
    for &(ref name, ref value) in pairs.iter() {
        if output.len() > 0 {
            output.push('&'.to_ascii());
            byte_serialize(name.as_slice(), &mut output, encoding_override);
            output.push('='.to_ascii());
            byte_serialize(value.as_slice(), &mut output, encoding_override);
        }
    }
    output
}

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
#![feature(macro_rules, default_type_params)]

extern crate encoding;

#[cfg(test)]
extern crate serialize;

use std::cmp;
use std::hash;
use std::str::from_utf8_lossy;
use std::ascii::OwnedStrAsciiExt;

use encoding::EncodingRef;

use encode_sets::{PASSWORD_ENCODE_SET, USERNAME_ENCODE_SET};


mod encode_sets;
mod parser;
pub mod form_urlencoded;
pub mod punycode;

#[cfg(test)]
mod tests;


pub struct Url {
    pub scheme: String,
    pub scheme_data: SchemeData,
    pub query: Option<String>,  // See form_urlencoded::parse_str() to get name/value pairs.
    pub fragment: Option<String>,
    encoding_override: Option<EncodingRef>,
}

#[deriving(PartialEq, Eq, Clone)]
pub enum SchemeData {
    RelativeSchemeData(SchemeRelativeUrl),
    OtherSchemeData(String),  // data: URLs, mailto: URLs, etc.
}

#[deriving(PartialEq, Eq, Clone)]
pub struct SchemeRelativeUrl {
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

impl Clone for Url {
    fn clone(&self) -> Url {
        Url {
            scheme: self.scheme.clone(),
            scheme_data: self.scheme_data.clone(),
            query: self.query.clone(),
            fragment: self.fragment.clone(),
            encoding_override: self.encoding_override,
        }
    }
}

impl Eq for Url {}

impl PartialEq for Url {
    fn eq(&self, other: &Url) -> bool {
        self.scheme == other.scheme &&
        self.scheme_data == other.scheme_data &&
        self.query == other.query &&
        self.fragment == other.fragment &&
        self.encoding_override.map(|e| e.name()) == other.encoding_override.map(|e| e.name())
    }
}

impl<S: hash::Writer> hash::Hash<S> for Url {
    fn hash(&self, state: &mut S) {
        self.serialize().hash(state)
    }
}

macro_rules! is_match(
    ($value:expr, $($pattern:pat)|+) => (
        match $value { $($pattern)|+ => true, _ => false }
    );
)


pub type ParseResult<T> = Result<T, &'static str>;

/// This is called on non-fatal parse errors.
/// The handler can choose to continue or abort parsing by returning Ok() or Err(), respectively.
/// FIXME: make this a by-ref closure when that’s supported.
pub type ErrorHandler = fn(reason: &'static str) -> ParseResult<()>;

fn silent_handler(_reason: &'static str) -> ParseResult<()> {
    Ok(())
}


impl Url {
    pub fn parse(input: &str, base_url: Option<&Url>)
                 -> ParseResult<Url> {
        let encoding_override = None;
        parser::parse_url(input, base_url, encoding_override, silent_handler)
    }

    pub fn serialize(&self) -> String {
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

    pub fn serialize_no_fragment(&self) -> String {
        let mut result = self.scheme.clone();
        result.push_str(":");
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl {
                ref username, ref password, ref host, ref port, ref path
            }) => {
                result.push_str("//");
                if !username.is_empty() || password.is_some() {
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
                result.push_str(host.serialize().as_slice());
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

impl UrlUtils for Url {
    /// `URLUtils.protocol` setter
    fn set_scheme(&mut self, input: &str) -> ParseResult<()> {
        match parser::parse_scheme(input.as_slice(), parser::SetterContext) {
            Some((scheme, _)) => {
                self.scheme = scheme;
                Ok(())
            },
            None => Err("Invalid scheme"),
        }
    }

    /// `URLUtils.username` setter
    fn set_username(&mut self, input: &str) -> ParseResult<()> {
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { ref mut username, .. }) => {
                username.truncate(0);
                utf8_percent_encode(input, USERNAME_ENCODE_SET, username);
                Ok(())
            },
            OtherSchemeData(_) => Err("Can not set username on non-relative URL.")
        }
    }

    /// `URLUtils.password` setter
    fn set_password(&mut self, input: &str) -> ParseResult<()> {
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { ref mut password, .. }) => {
                let mut new_password = String::new();
                utf8_percent_encode(input, PASSWORD_ENCODE_SET, &mut new_password);
                *password = Some(new_password);
                Ok(())
            },
            OtherSchemeData(_) => Err("Can not set password on non-relative URL.")
        }
    }

    /// `URLUtils.host` setter
    fn set_host_and_port(&mut self, input: &str) -> ParseResult<()> {
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { ref mut host, ref mut port, .. }) => {
                let (new_host, new_port, _) = try!(parser::parse_host(
                    input, self.scheme.as_slice(), silent_handler));
                *host = new_host;
                *port = new_port;
                Ok(())
            },
            OtherSchemeData(_) => Err("Can not set host/port on non-relative URL.")
        }
    }

    /// `URLUtils.hostname` setter
    fn set_host(&mut self, input: &str) -> ParseResult<()> {
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { ref mut host, .. }) => {
                let (new_host, _) = try!(parser::parse_hostname(
                    input, silent_handler));
                *host = new_host;
                Ok(())
            },
            OtherSchemeData(_) => Err("Can not set host on non-relative URL.")
        }
    }

    /// `URLUtils.port` setter
    fn set_port(&mut self, input: &str) -> ParseResult<()> {
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { ref mut port, .. }) => {
                if self.scheme.as_slice() == "file" {
                    return Err("Can not set port on file: URL.")
                }
                let (new_port, _) = try!(parser::parse_port(
                    input, self.scheme.as_slice(), silent_handler));
                *port = new_port;
                Ok(())
            },
            OtherSchemeData(_) => Err("Can not set port on non-relative URL.")
        }
    }

    /// `URLUtils.pathname` setter
    fn set_path(&mut self, input: &str) -> ParseResult<()> {
        match self.scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { ref mut path, .. }) => {
                let (new_path, _) = try!(parser::parse_path_start(
                    input, parser::SetterContext,
                    if self.scheme.as_slice() == "file" { parser::FileScheme }
                        else { parser::NonFileScheme },
                    silent_handler));
                *path = new_path;
                Ok(())
            },
            OtherSchemeData(_) => Err("Can not set path on non-relative URL.")
        }
    }

    /// `URLUtils.search` setter
    fn set_query(&mut self, input: &str) -> ParseResult<()> {
        // FIXME: This is in the spec, but seems superfluous.
        match self.scheme_data {
            RelativeSchemeData(_) => (),
            OtherSchemeData(_) => return Err("Can not set query on non-relative URL.")
        }
        self.query = if input.is_empty() {
            None
        } else {
            let input = if input.starts_with("?") { input.slice_from(1) } else { input };
            let (new_query, _) = try!(parser::parse_query(
                input, self.encoding_override, parser::SetterContext, silent_handler));
            Some(new_query)
        };
        Ok(())
    }

    /// `URLUtils.hash` setter
    fn set_fragment(&mut self, input: &str) -> ParseResult<()> {
        if self.scheme.as_slice() == "javascript" {
            return Err("Can not set fragment on a javascript: URL.")
        }
        self.fragment = if input.is_empty() {
            None
        } else {
            let input = if input.starts_with("#") { input.slice_from(1) } else { input };
            Some(try!(parser::parse_fragment(input, silent_handler)))
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
            let domain = from_utf8_lossy(decoded.as_slice());
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
        match *self {
            Domain(ref domain) => domain.clone(),
            Ipv6(ref address) => {
                let mut result = String::from_str("[");
                result.push_str(address.serialize().as_slice());
                result.push_str("]");
                result
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
        let mut output = String::new();
        let (compress_start, compress_end) = longest_zero_sequence(&self.pieces);
        let mut i = 0;
        while i < 8 {
            if i == compress_start {
                output.push_str(":");
                if i == 0 {
                    output.push_str(":");
                }
                if compress_end < 8 {
                    i = compress_end;
                } else {
                    break;
                }
            }
            output.push_str(format!("{:X}", self.pieces[i as uint]).as_slice());
            if i < 7 {
                output.push_str(":");
            }
            i += 1;
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

#[inline]
fn to_hex_upper(value: u8) -> u8 {
    match value {
        0 .. 9 => b'0' + value,
        10 .. 15 => b'A' + value - 10,
        _ => fail!()
    }
}


#[inline]
pub fn utf8_percent_encode(input: &str, encode_set: &[&str], output: &mut String) {
    percent_encode(input.as_bytes(), encode_set, output)
}


#[inline]
pub fn percent_encode(input: &[u8], encode_set: &[&str], output: &mut String) {
    for &byte in input.iter() {
        output.push_str(encode_set[byte as uint])
    }
}


#[inline]
pub fn percent_encode_byte(byte: u8, output: &mut String) {
    unsafe {
        output.push_bytes([
            b'%', to_hex_upper(byte >> 4), to_hex_upper(byte & 0x0F)
        ])
    }
}


#[inline]
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
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
    output
}

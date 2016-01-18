// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ascii::AsciiExt;
use std::cmp;
use std::fmt::{self, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use parser::{ParseResult, ParseError};
use percent_encoding::{from_hex, percent_decode};
use idna;


/// The host name of an URL.
#[derive(PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord)]
#[cfg_attr(feature="heap_size", derive(HeapSizeOf))]
pub enum Host {
    /// A (DNS) domain name.
    Domain(String),
    /// A IPv4 address, represented by four sequences of up to three ASCII digits.
    Ipv4(Ipv4Addr),
    /// An IPv6 address, represented inside `[...]` square brackets
    /// so that `:` colon characters in the address are not ambiguous
    /// with the port number delimiter.
    Ipv6(Ipv6Addr),
}


impl Host {
    /// Parse a host: either an IPv6 address in [] square brackets, or a domain.
    ///
    /// Returns `Err` for an empty host, an invalid IPv6 address,
    /// or a or invalid non-ASCII domain.
    pub fn parse(input: &str) -> ParseResult<Host> {
        if input.len() == 0 {
            return Err(ParseError::EmptyHost)
        }
        if input.starts_with("[") {
            if !input.ends_with("]") {
                return Err(ParseError::InvalidIpv6Address)
            }
            return parse_ipv6addr(&input[1..input.len() - 1]).map(Host::Ipv6)
        }
        let decoded = percent_decode(input.as_bytes());
        let domain = String::from_utf8_lossy(&decoded);

        let domain = match idna::domain_to_ascii(&domain) {
            Ok(s) => s,
            Err(_) => return Err(ParseError::InvalidDomainCharacter)
        };

        if domain.find(&[
            '\0', '\t', '\n', '\r', ' ', '#', '%', '/', ':', '?', '@', '[', '\\', ']'
        ][..]).is_some() {
            return Err(ParseError::InvalidDomainCharacter)
        }
        match parse_ipv4addr(&domain[..]) {
            Ok(Some(ipv4addr)) => Ok(Host::Ipv4(ipv4addr)),
            Ok(None) => Ok(Host::Domain(domain.to_ascii_lowercase())),
            Err(e) => Err(e),
        }
    }

    /// Serialize the host as a string.
    ///
    /// A domain a returned as-is, an IPv6 address between [] square brackets.
    pub fn serialize(&self) -> String {
        self.to_string()
    }
}


impl fmt::Display for Host {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Host::Domain(ref domain) => domain.fmt(f),
            Host::Ipv4(ref addr) => addr.fmt(f),
            Host::Ipv6(ref addr) => {
                try!(f.write_str("["));
                try!(write_ipv6(addr, f));
                f.write_str("]")
            }
        }
    }
}

fn write_ipv6(addr: &Ipv6Addr, f: &mut Formatter) -> fmt::Result {
    let segments = addr.segments();
    let (compress_start, compress_end) = longest_zero_sequence(&segments);
    let mut i = 0;
    while i < 8 {
        if i == compress_start {
            try!(f.write_str(":"));
            if i == 0 {
                try!(f.write_str(":"));
            }
            if compress_end < 8 {
                i = compress_end;
            } else {
                break;
            }
        }
        try!(write!(f, "{:x}", segments[i as usize]));
        if i < 7 {
            try!(f.write_str(":"));
        }
        i += 1;
    }
    Ok(())
}

fn longest_zero_sequence(pieces: &[u16; 8]) -> (isize, isize) {
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
    for i in 0..8 {
        if pieces[i as usize] == 0 {
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


fn parse_ipv4number(mut input: &str) -> ParseResult<u32> {
    let mut r = 10;
    if input.starts_with("0x") || input.starts_with("0X") {
        input = &input[2..];
        r = 16;
    } else if input.len() >= 2 && input.starts_with("0") {
        input = &input[1..];
        r = 8;
    }
    if input.is_empty() {
        return Ok(0);
    }
    if input.starts_with("+") {
        return Err(ParseError::InvalidIpv4Address)
    }
    match u32::from_str_radix(&input, r) {
        Ok(number) => Ok(number),
        Err(_) => Err(ParseError::InvalidIpv4Address),
    }
}

fn parse_ipv4addr(input: &str) -> ParseResult<Option<Ipv4Addr>> {
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    if parts.len() > 4 {
        return Ok(None);
    }
    let mut numbers: Vec<u32> = Vec::new();
    for part in parts {
        if part == "" {
            return Ok(None);
        }
        if let Ok(n) = parse_ipv4number(part) {
            numbers.push(n);
        } else {
            return Ok(None);
        }
    }
    let mut ipv4 = numbers.pop().expect("a non-empty list of numbers");
    // Equivalent to: ipv4 >= 256 ** (4 − numbers.len())
    if ipv4 > u32::max_value() >> (8 * numbers.len() as u32)  {
        return Err(ParseError::InvalidIpv4Address);
    }
    if numbers.iter().any(|x| *x > 255) {
        return Err(ParseError::InvalidIpv4Address);
    }
    for (counter, n) in numbers.iter().enumerate() {
        ipv4 += n << (8 * (3 - counter as u32))
    }
    Ok(Some(Ipv4Addr::from(ipv4)))
}


fn parse_ipv6addr(input: &str) -> ParseResult<Ipv6Addr> {
    let input = input.as_bytes();
    let len = input.len();
    let mut is_ip_v4 = false;
    let mut pieces = [0, 0, 0, 0, 0, 0, 0, 0];
    let mut piece_pointer = 0;
    let mut compress_pointer = None;
    let mut i = 0;

    if len < 2 {
        return Err(ParseError::InvalidIpv6Address)
    }

    if input[0] == b':' {
        if input[1] != b':' {
            return Err(ParseError::InvalidIpv6Address)
        }
        i = 2;
        piece_pointer = 1;
        compress_pointer = Some(1);
    }

    while i < len {
        if piece_pointer == 8 {
            return Err(ParseError::InvalidIpv6Address)
        }
        if input[i] == b':' {
            if compress_pointer.is_some() {
                return Err(ParseError::InvalidIpv6Address)
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
                        return Err(ParseError::InvalidIpv6Address)
                    }
                    i = start;
                    is_ip_v4 = true;
                },
                b':' => {
                    i += 1;
                    if i == len {
                        return Err(ParseError::InvalidIpv6Address)
                    }
                },
                _ => return Err(ParseError::InvalidIpv6Address)
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
            return Err(ParseError::InvalidIpv6Address)
        }
        let mut dots_seen = 0;
        while i < len {
            // FIXME: https://github.com/whatwg/url/commit/1c22aa119c354e0020117e02571cec53f7c01064
            let mut value = 0u16;
            while i < len {
                let digit = match input[i] {
                    c @ b'0' ... b'9' => c - b'0',
                    _ => break
                };
                value = value * 10 + digit as u16;
                if value == 0 || value > 255 {
                    return Err(ParseError::InvalidIpv6Address)
                }
            }
            if dots_seen < 3 && !(i < len && input[i] == b'.') {
                return Err(ParseError::InvalidIpv6Address)
            }
            pieces[piece_pointer] = pieces[piece_pointer] * 0x100 + value;
            if dots_seen == 0 || dots_seen == 2 {
                piece_pointer += 1;
            }
            i += 1;
            if dots_seen == 3 && i < len {
                return Err(ParseError::InvalidIpv6Address)
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
            return Err(ParseError::InvalidIpv6Address)
        }
    }
    Ok(Ipv6Addr::new(pieces[0], pieces[1], pieces[2], pieces[3],
                     pieces[4], pieces[5], pieces[6], pieces[7]))
}

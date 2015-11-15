// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ascii::AsciiExt;
use std::fmt::{self, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};
use parser::{ParseResult, ParseError};
use percent_encoding::{percent_decode};


/// The host name of an URL.
#[derive(PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord)]
#[cfg_attr(feature="heap_size", derive(HeapSizeOf))]
pub enum Host {
    /// A (DNS) domain name.
    Domain(String),
    /// An IPv4 address.
    V4(Ipv4Addr),
    /// An IPv6 address.
    V6(Ipv6Addr),
}

impl Host {
    /// Parse a host: either an IPv6 address in [] square brackets, or a domain.
    ///
    /// Returns `Err` for an empty host, an invalid IPv6 address,
    /// or a or invalid non-ASCII domain.
    ///
    /// FIXME: Add IDNA support for non-ASCII domains.
    pub fn parse(input: &str) -> ParseResult<Host> {
        if input.len() == 0 {
            Err(ParseError::EmptyHost)
        } else if input.starts_with("[") {
            if input.ends_with("]") {
                if let Ok(addr) = input[1..input.len() - 1].parse() {
                    Ok(Host::V6(addr))
                } else {
                    Err(ParseError::InvalidIpv6Address)
                }
            } else {
                Err(ParseError::InvalidIpv6Address)
            }
        } else {
            if let Ok(addr) = input.parse() {
                Ok(Host::V4(addr))
            } else {
                let decoded = percent_decode(input.as_bytes());
                let domain = String::from_utf8_lossy(&decoded);
                // TODO: Remove this check and use IDNA "domain to ASCII"
                if !domain.is_ascii() {
                    Err(ParseError::NonAsciiDomainsNotSupportedYet)
                } else if domain.find(&[
                    '\0', '\t', '\n', '\r', ' ', '#', '%', '/', ':', '?', '@', '[', '\\', ']'
                ][..]).is_some() {
                    Err(ParseError::InvalidDomainCharacter)
                } else {
                    Ok(Host::Domain(domain.to_ascii_lowercase()))
                }
            }
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
            Host::V4(ref addr) => addr.fmt(f),
            Host::V6(ref addr) => write!(f, "[{}]", addr),
        }
    }
}

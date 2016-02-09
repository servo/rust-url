// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::fmt::{self, Write};
use std::slice;

/// Represents a set of characters / bytes that should be percent-encoded.
///
/// See [encode sets specification](http://url.spec.whatwg.org/#simple-encode-set).
///
/// Different characters need to be encoded in different parts of an URL.
/// For example, a literal `?` question mark in an URL’s path would indicate
/// the start of the query string.
/// A question mark meant to be part of the path therefore needs to be percent-encoded.
/// In the query string however, a question mark does not have any special meaning
/// and does not need to be percent-encoded.
///
/// A few sets are defined in this module.
/// Use the [`define_encode_set!`](../macro.define_encode_set!.html) macro to define different ones.
pub trait EncodeSet: Clone {
    /// Called with UTF-8 bytes rather than code points.
    /// Should return false for all non-ASCII bytes.
    fn contains(&self, byte: u8) -> bool;
}

/// Define a new struct
/// that implements the [`EncodeSet`](percent_encoding/trait.EncodeSet.html) trait,
/// for use in [`percent_decode()`](percent_encoding/fn.percent_encode.html)
/// and related functions.
///
/// Parameters are characters to include in the set in addition to those of the base set.
/// See [encode sets specification](http://url.spec.whatwg.org/#simple-encode-set).
///
/// Example
/// =======
///
/// ```rust
/// #[macro_use] extern crate url;
/// use url::percent_encoding::{utf8_percent_encode, SIMPLE_ENCODE_SET};
/// define_encode_set! {
///     /// This encode set is used in the URL parser for query strings.
///     pub QUERY_ENCODE_SET = [SIMPLE_ENCODE_SET] | {' ', '"', '#', '<', '>'}
/// }
/// # fn main() {
/// assert_eq!(utf8_percent_encode("foo bar", QUERY_ENCODE_SET).collect::<String>(), "foo%20bar");
/// # }
/// ```
#[macro_export]
macro_rules! define_encode_set {
    ($(#[$attr: meta])* pub $name: ident = [$base_set: expr] | {$($ch: pat),*}) => {
        $(#[$attr])*
        #[derive(Copy, Clone)]
        #[allow(non_camel_case_types)]
        pub struct $name;

        impl $crate::percent_encoding::EncodeSet for $name {
            #[inline]
            fn contains(&self, byte: u8) -> bool {
                match byte as char {
                    $(
                        $ch => true,
                    )*
                    _ => $base_set.contains(byte)
                }
            }
        }
    }
}

/// This encode set is used for fragment identifier and non-relative scheme data.
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
pub struct SIMPLE_ENCODE_SET;

impl EncodeSet for SIMPLE_ENCODE_SET {
    #[inline]
    fn contains(&self, byte: u8) -> bool {
        byte < 0x20 || byte > 0x7E
    }
}

define_encode_set! {
    /// This encode set is used in the URL parser for query strings.
    pub QUERY_ENCODE_SET = [SIMPLE_ENCODE_SET] | {' ', '"', '#', '<', '>'}
}

define_encode_set! {
    /// This encode set is used for path components.
    pub DEFAULT_ENCODE_SET = [QUERY_ENCODE_SET] | {'`', '?', '{', '}'}
}

define_encode_set! {
    /// This encode set is used for username and password.
    pub PATH_SEGMENT_ENCODE_SET = [DEFAULT_ENCODE_SET] | {'%'}
}

define_encode_set! {
    /// This encode set is used for username and password.
    pub USERINFO_ENCODE_SET = [DEFAULT_ENCODE_SET] | {
        '/', ':', ';', '=', '@', '[', '\\', ']', '^', '|'
    }
}

define_encode_set! {
    /// This encode set is used in `application/x-www-form-urlencoded` serialization.
    pub FORM_URLENCODED_ENCODE_SET = [SIMPLE_ENCODE_SET] | {
        ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '+', ',', '/', ':', ';',
        '<', '=', '>', '?', '@', '[', '\\', ']', '^', '`', '{', '|', '}', '~'
    }
}

/// Percent-encode the given bytes and return an iterator of `char` in the ASCII range.
#[inline]
pub fn percent_encode<E: EncodeSet>(input: &[u8], encode_set: E) -> PercentEncode<E> {
    PercentEncode {
        iter: input.iter(),
        encode_set: encode_set,
        state: PercentEncodeState::NextByte,
    }
}

/// Percent-encode the UTF-8 encoding of the given string
/// and return an iterator of `char` in the ASCII range.
#[inline]
pub fn utf8_percent_encode<E: EncodeSet>(input: &str, encode_set: E) -> PercentEncode<E> {
    percent_encode(input.as_bytes(), encode_set)
}

#[derive(Clone)]
pub struct PercentEncode<'a, E: EncodeSet> {
    iter: slice::Iter<'a, u8>,
    encode_set: E,
    state: PercentEncodeState,
}

#[derive(Clone)]
enum PercentEncodeState {
    NextByte,
    HexHigh(u8),
    HexLow(u8),
}

impl<'a, E: EncodeSet> Iterator for PercentEncode<'a, E> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        // str::char::from_digit always returns lowercase.
        const UPPER_HEX: [char; 16] = ['0', '1', '2', '3', '4', '5', '6', '7',
                                       '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'];
        match self.state {
            PercentEncodeState::HexHigh(byte) => {
                self.state = PercentEncodeState::HexLow(byte);
                Some(UPPER_HEX[(byte >> 4) as usize])
            }
            PercentEncodeState::HexLow(byte) => {
                self.state = PercentEncodeState::NextByte;
                Some(UPPER_HEX[(byte & 0x0F) as usize])
            }
            PercentEncodeState::NextByte => {
                self.iter.next().map(|&byte| {
                    if self.encode_set.contains(byte) {
                        self.state = PercentEncodeState::HexHigh(byte);
                        '%'
                    } else {
                        assert!(byte.is_ascii());
                        byte as char
                    }
                })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self.iter.size_hint();
        (low.saturating_add(2) / 3, high)
    }
}

impl<'a, E: EncodeSet> fmt::Display for PercentEncode<'a, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for c in (*self).clone() {
            try!(formatter.write_char(c))
        }
        Ok(())
    }
}

/// Percent-decode the given bytes and return an iterator of bytes.
#[inline]
pub fn percent_decode(input: &[u8]) -> PercentDecode {
    PercentDecode {
        iter: input.iter()
    }
}

#[derive(Clone)]
pub struct PercentDecode<'a> {
    iter: slice::Iter<'a, u8>,
}

impl<'a> Iterator for PercentDecode<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.iter.next().map(|&byte| {
            if byte == b'%' {
                let after_percent_sign = self.iter.clone();
                let h = self.iter.next().and_then(|&b| (b as char).to_digit(16));
                let l = self.iter.next().and_then(|&b| (b as char).to_digit(16));
                if let (Some(h), Some(l)) = (h, l) {
                    return h as u8 * 0x10 + l as u8
                }
                self.iter = after_percent_sign;
            }
            byte
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (low, high) = self.iter.size_hint();
        (low, high.and_then(|high| high.checked_mul(3)))
    }
}

/// Percent-decode the given bytes, and decode the result as UTF-8.
///
/// This is return `Err` when the percent-decoded bytes are not well-formed in UTF-8.
pub fn utf8_percent_decode(input: &[u8]) -> Result<String, ::std::string::FromUtf8Error> {
    let bytes = percent_decode(input).collect::<Vec<u8>>();
    String::from_utf8(bytes)
}

/// Percent-decode the given bytes, and decode the result as UTF-8.
///
/// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
/// will be replaced � U+FFFD, the replacement character.
pub fn lossy_utf8_percent_decode(input: &[u8]) -> String {
    let bytes = percent_decode(input).collect::<Vec<u8>>();
    match String::from_utf8_lossy(&bytes) {
        Cow::Owned(s) => return s,
        Cow::Borrowed(_) => {}
    }
    unsafe {
        String::from_utf8_unchecked(bytes)
    }
}

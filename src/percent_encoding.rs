// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ascii::AsciiExt;
use std::fmt::Write;

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
pub trait EncodeSet {
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
/// assert_eq!(utf8_percent_encode("foo bar", QUERY_ENCODE_SET), "foo%20bar");
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
    /// This encode set is used in the URL parser for usernames and passwords.
    pub USERINFO_ENCODE_SET = [DEFAULT_ENCODE_SET] | {'@'}
}

define_encode_set! {
    /// This encode set should be used when setting the password field of a parsed URL.
    pub PASSWORD_ENCODE_SET = [USERINFO_ENCODE_SET] | {'\\', '/'}
}

define_encode_set! {
    /// This encode set should be used when setting the username field of a parsed URL.
    pub USERNAME_ENCODE_SET = [PASSWORD_ENCODE_SET] | {':'}
}

define_encode_set! {
    /// This encode set is used in `application/x-www-form-urlencoded` serialization.
    pub FORM_URLENCODED_ENCODE_SET = [SIMPLE_ENCODE_SET] | {
        ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '+', ',', '/', ':', ';',
        '<', '=', '>', '?', '@', '[', '\\', ']', '^', '`', '{', '|', '}', '~'
    }
}

define_encode_set! {
    /// This encode set is used for HTTP header values and is defined at
    /// https://tools.ietf.org/html/rfc5987#section-3.2
    pub HTTP_VALUE = [SIMPLE_ENCODE_SET] | {
        ' ', '"', '%', '\'', '(', ')', '*', ',', '/', ':', ';', '<', '-', '>', '?',
        '[', '\\', ']', '{', '}'
    }
}

/// Percent-encode the given bytes, and push the result to `output`.
///
/// The pushed strings are within the ASCII range.
#[inline]
pub fn percent_encode_to<E: EncodeSet>(input: &[u8], encode_set: E, output: &mut String) {
    for &byte in input {
        if encode_set.contains(byte) {
            write!(output, "%{:02X}", byte).unwrap();
        } else {
            assert!(byte.is_ascii());
            unsafe {
                output.as_mut_vec().push(byte)
            }
        }
    }
}


/// Percent-encode the given bytes.
///
/// The returned string is within the ASCII range.
#[inline]
pub fn percent_encode<E: EncodeSet>(input: &[u8], encode_set: E) -> String {
    let mut output = String::new();
    percent_encode_to(input, encode_set, &mut output);
    output
}


/// Percent-encode the UTF-8 encoding of the given string, and push the result to `output`.
///
/// The pushed strings are within the ASCII range.
#[inline]
pub fn utf8_percent_encode_to<E: EncodeSet>(input: &str, encode_set: E, output: &mut String) {
    percent_encode_to(input.as_bytes(), encode_set, output)
}


/// Percent-encode the UTF-8 encoding of the given string.
///
/// The returned string is within the ASCII range.
#[inline]
pub fn utf8_percent_encode<E: EncodeSet>(input: &str, encode_set: E) -> String {
    let mut output = String::new();
    utf8_percent_encode_to(input, encode_set, &mut output);
    output
}


/// Percent-decode the given bytes, and push the result to `output`.
pub fn percent_decode_to(input: &[u8], output: &mut Vec<u8>) {
    let mut i = 0;
    while i < input.len() {
        let c = input[i];
        if c == b'%' && i + 2 < input.len() {
            if let (Some(h), Some(l)) = (from_hex(input[i + 1]), from_hex(input[i + 2])) {
                output.push(h * 0x10 + l);
                i += 3;
                continue
            }
        }

        output.push(c);
        i += 1;
    }
}


/// Percent-decode the given bytes.
#[inline]
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    percent_decode_to(input, &mut output);
    output
}


/// Percent-decode the given bytes, and decode the result as UTF-8.
///
/// This is “lossy”: invalid UTF-8 percent-encoded byte sequences
/// will be replaced � U+FFFD, the replacement character.
#[inline]
pub fn lossy_utf8_percent_decode(input: &[u8]) -> String {
    String::from_utf8_lossy(&percent_decode(input)).to_string()
}

#[inline]
pub fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0' ... b'9' => Some(byte - b'0'),  // 0..9
        b'A' ... b'F' => Some(byte + 10 - b'A'),  // A..F
        b'a' ... b'f' => Some(byte + 10 - b'a'),  // a..f
        _ => None
    }
}

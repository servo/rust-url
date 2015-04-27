// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[path = "encode_sets.rs"]
mod encode_sets;

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
/// Since the implementation details of `EncodeSet` are private,
/// the set of available encode sets is not extensible beyond the ones
/// provided here.
/// If you need a different encode set,
/// please [file a bug](https://github.com/servo/rust-url/issues)
/// explaining the use case.
#[derive(Copy, Clone)]
pub struct EncodeSet {
    map: &'static [&'static str; 256],
}

/// This encode set is used for fragment identifier and non-relative scheme data.
pub static SIMPLE_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::SIMPLE };

/// This encode set is used in the URL parser for query strings.
pub static QUERY_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::QUERY };

/// This encode set is used for path components.
pub static DEFAULT_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::DEFAULT };

/// This encode set is used in the URL parser for usernames and passwords.
pub static USERINFO_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::USERINFO };

/// This encode set should be used when setting the password field of a parsed URL.
pub static PASSWORD_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::PASSWORD };

/// This encode set should be used when setting the username field of a parsed URL.
pub static USERNAME_ENCODE_SET: EncodeSet = EncodeSet { map: &encode_sets::USERNAME };

/// This encode set is used in `application/x-www-form-urlencoded` serialization.
pub static FORM_URLENCODED_ENCODE_SET: EncodeSet = EncodeSet {
    map: &encode_sets::FORM_URLENCODED,
};


/// Percent-encode the given bytes, and push the result to `output`.
///
/// The pushed strings are within the ASCII range.
#[inline]
pub fn percent_encode_to(input: &[u8], encode_set: EncodeSet, output: &mut String) {
    for &byte in input {
        output.push_str(encode_set.map[byte as usize])
    }
}


/// Percent-encode the given bytes.
///
/// The returned string is within the ASCII range.
#[inline]
pub fn percent_encode(input: &[u8], encode_set: EncodeSet) -> String {
    let mut output = String::new();
    percent_encode_to(input, encode_set, &mut output);
    output
}


/// Percent-encode the UTF-8 encoding of the given string, and push the result to `output`.
///
/// The pushed strings are within the ASCII range.
#[inline]
pub fn utf8_percent_encode_to(input: &str, encode_set: EncodeSet, output: &mut String) {
    percent_encode_to(input.as_bytes(), encode_set, output)
}


/// Percent-encode the UTF-8 encoding of the given string.
///
/// The returned string is within the ASCII range.
#[inline]
pub fn utf8_percent_encode(input: &str, encode_set: EncodeSet) -> String {
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

// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// Parser and serializer for `application/x-www-form-urlencoded`
///
/// Converts between a string (such as an URL’s query string)
/// and a list of name/value pairs.

use std::str;

use encoding;
use encoding::EncodingRef;
use encoding::all::UTF_8;
use encoding::label::encoding_from_whatwg_label;

use super::{percent_encode_byte, percent_decode};


pub fn parse_str(input: &str) -> Vec<(String, String)> {
    parse_bytes(input.as_bytes(), None, false, false).unwrap()
}


pub fn parse_bytes(input: &[u8], encoding_override: Option<EncodingRef>,
                   mut use_charset: bool, mut isindex: bool) -> Option<Vec<(String, String)>> {
    let mut encoding_override = encoding_override.unwrap_or(UTF_8 as EncodingRef);
    let mut pairs = Vec::new();
    for piece in input.split(|&b| b == b'&') {
        if piece.is_empty() {
            if isindex {
                pairs.push((Vec::new(), Vec::new()))
            }
        } else {
            let (name, value) = match piece.position_elem(&b'=') {
                Some(position) => (piece.slice_to(position), piece.slice_from(position + 1)),
                None => if isindex { (&[], piece) } else { (piece, &[]) }
            };
            let name = replace_plus(name);
            let value = replace_plus(value);
            if use_charset && name.as_slice() == b"_charset_" {
                // Non-UTF8 here is ok, encoding_from_whatwg_label only matches in the ASCII range.
                match encoding_from_whatwg_label(unsafe { str::raw::from_utf8(value.as_slice()) }) {
                    Some(encoding) => encoding_override = encoding,
                    None => (),
                }
                use_charset = false;
            }
            pairs.push((name, value));
        }
        isindex = false;
    }
    if encoding_override.name() != "utf-8" && !input.is_ascii() {
        return None
    }

    #[inline]
    fn replace_plus(input: &[u8]) -> Vec<u8> {
        input.iter().map(|&b| if b == b'+' { b' ' } else { b }).collect()
    }

    #[inline]
    fn decode(input: Vec<u8>, encoding_override: EncodingRef) -> String {
        let mut bytes = Vec::new();
        percent_decode(input.as_slice(), &mut bytes);
        encoding_override.decode(bytes.as_slice(), encoding::DecodeReplace).unwrap()
    }

    Some(pairs.move_iter().map(
        |(name, value)| (decode(name, encoding_override), decode(value, encoding_override))
    ).collect())
}


pub fn serialize<'a, I: Iterator<(&'a str, &'a str)>>(
        mut pairs: I, encoding_override: Option<EncodingRef>)
        -> String {
    #[inline]
    fn byte_serialize(input: &str, output: &mut String,
                     encoding_override: Option<EncodingRef>) {
        let keep_alive;
        let input = match encoding_override {
            None => input.as_bytes(),  // "Encode" to UTF-8
            Some(encoding) => {
                keep_alive = encoding.encode(input, encoding::EncodeNcrEscape).unwrap();
                keep_alive.as_slice()
            }
        };

        for &byte in input.iter() {
            match byte {
                b' ' => output.push_str("+"),
                b'*' | b'-' | b'.' | b'0' .. b'9' | b'A' .. b'Z' | b'_' | b'a' .. b'z'
                => unsafe { output.push_byte(byte) },
                _ => percent_encode_byte(byte, output),
            }
        }
    }

    let mut output = String::new();
    for (name, value) in pairs {
        if output.len() > 0 {
            output.push_str("&");
            byte_serialize(name, &mut output, encoding_override);
            output.push_str("=");
            byte_serialize(value, &mut output, encoding_override);
        }
    }
    output
}

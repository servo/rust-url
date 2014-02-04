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


pub fn parse_str(input: &str) -> ~[(~str, ~str)] {
    parse_bytes(input.as_bytes(), None, false, false).unwrap()
}


pub fn parse_bytes(input: &[u8], encoding_override: Option<EncodingRef>,
                   mut use_charset: bool, mut isindex: bool) -> Option<~[(~str, ~str)]> {
    let mut encoding_override = encoding_override.unwrap_or(UTF_8 as EncodingRef);
    let mut pairs = ~[];
    for piece in input.split(|&b| b == '&' as u8) {
        if piece.is_empty() {
            if isindex {
                pairs.push((~[], ~[]))
            }
        } else {
            let (name, value) = match piece.position_elem(&('=' as u8)) {
                Some(position) => (piece.slice_to(position), piece.slice_from(position + 1)),
                None => if isindex { (&[], piece) } else { (piece, &[]) }
            };
            let name = replace_plus(name);
            let value = replace_plus(value);
            if use_charset && name.as_slice() == "_charset_".as_bytes() {
                // Non-UTF8 here is ok, encoding_from_whatwg_label only matches in the ASCII range.
                match encoding_from_whatwg_label(unsafe { str::raw::from_utf8(value) }) {
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
    fn replace_plus(input: &[u8]) -> ~[u8] {
        input.iter().map(|&b| if b == '+' as u8 { ' ' as u8 } else { b }).to_owned_vec()
    }

    #[inline]
    fn decode(input: ~[u8], encoding_override: EncodingRef) -> ~str {
        let bytes = percent_decode(input.as_slice());
        encoding_override.decode(bytes, encoding::DecodeReplace).unwrap()
    }

    Some(pairs.move_iter().map(
        |(name, value)| (decode(name, encoding_override), decode(value, encoding_override))
    ).to_owned_vec())
}


pub fn serialize_form_urlencoded(pairs: ~[(~str, ~str)],
                                 encoding_override: Option<EncodingRef>)
                              -> ~str {
    #[inline]
    fn byte_serialize(input: &str, output: &mut ~str,
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
                0x20 => output.push_str("+"),
                0x2A | 0x2D | 0x2E | 0x30 .. 0x39 | 0x41 .. 0x5A | 0x5F | 0x61 .. 0x7A
                => unsafe { str::raw::push_byte(output, *byte) },
                _ => percent_encode_byte(*byte, output),
            }
        }
    }

    let mut output = ~"";
    for &(ref name, ref value) in pairs.iter() {
        if output.len() > 0 {
            output.push_str("&");
            byte_serialize(name.as_slice(), &mut output, encoding_override);
            output.push_str("=");
            byte_serialize(value.as_slice(), &mut output, encoding_override);
        }
    }
    output
}

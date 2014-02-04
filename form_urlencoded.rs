// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::str;

use encoding;
use encoding::EncodingRef;
use encoding::all::UTF_8;
use encoding::label::encoding_from_whatwg_label;

use super::{percent_encode_byte, percent_decode};


pub fn parse_form_urlencoded(input: &str,
                             encoding_override: Option<EncodingRef>,
                             use_charset: bool,
                             mut isindex: bool)
                          -> ~[(~str, ~str)] {
    let mut encoding_override = encoding_override.unwrap_or(UTF_8 as EncodingRef);
    let mut pairs = ~[];
    for string in input.split('&') {
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
    fn decode(input: &~str, encoding_override: EncodingRef) -> ~str {
        let bytes = percent_decode(input.as_bytes());
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

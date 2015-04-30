// Copyright 2013-2015 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Parser and serializer for the [`application/x-www-form-urlencoded` format](
//! http://url.spec.whatwg.org/#application/x-www-form-urlencoded),
//! as used by HTML forms.
//!
//! Converts between a string (such as an URL’s query string)
//! and a sequence of (name, value) pairs.

use std::borrow::Borrow;
use std::ascii::AsciiExt;
use encoding::EncodingOverride;
use percent_encoding::{percent_encode_to, percent_decode, FORM_URLENCODED_ENCODE_SET};


/// Convert a byte string in the `application/x-www-form-urlencoded` format
/// into a vector of (name, value) pairs.
///
/// Use `parse(input.as_bytes())` to parse a `&str` string.
#[inline]
pub fn parse(input: &[u8]) -> Vec<(String, String)> {
    parse_internal(input, EncodingOverride::utf8(), false).unwrap()
}


/// Convert a byte string in the `application/x-www-form-urlencoded` format
/// into a vector of (name, value) pairs.
///
/// Use `parse(input.as_bytes())` to parse a `&str` string.
///
/// This function is only available if the `query_encoding` Cargo feature is enabled.
///
/// Arguments:
///
/// * `encoding_override`: The character encoding each name and values is decoded as
///    after percent-decoding. Defaults to UTF-8.
/// * `use_charset`: The *use _charset_ flag*. If in doubt, set to `false`.
#[cfg(feature = "query_encoding")]
#[inline]
pub fn parse_with_encoding(input: &[u8], encoding_override: Option<::encoding::EncodingRef>,
                           use_charset: bool)
                           -> Option<Vec<(String, String)>> {
    parse_internal(input, EncodingOverride::from_opt_encoding(encoding_override), use_charset)
}


fn parse_internal(input: &[u8], mut encoding_override: EncodingOverride, mut use_charset: bool)
                  -> Option<Vec<(String, String)>> {
    let mut pairs = Vec::new();
    for piece in input.split(|&b| b == b'&') {
        if !piece.is_empty() {
            let (name, value) = match piece.iter().position(|b| *b == b'=') {
                Some(position) => (&piece[..position], &piece[position + 1..]),
                None => (piece, &[][..])
            };

            #[inline]
            fn replace_plus(input: &[u8]) -> Vec<u8> {
                input.iter().map(|&b| if b == b'+' { b' ' } else { b }).collect()
            }

            let name = replace_plus(name);
            let value = replace_plus(value);
            if use_charset && name == b"_charset_" {
                if let Some(encoding) = EncodingOverride::lookup(&value) {
                    encoding_override = encoding;
                }
                use_charset = false;
            }
            pairs.push((name, value));
        }
    }
    if !(encoding_override.is_utf8() || input.is_ascii()) {
        return None
    }

    Some(pairs.into_iter().map(|(name, value)| (
        encoding_override.decode(&percent_decode(&name)),
        encoding_override.decode(&percent_decode(&value))
    )).collect())
}


/// Convert an iterator of (name, value) pairs
/// into a string in the `application/x-www-form-urlencoded` format.
#[inline]
pub fn serialize<I, K, V>(pairs: I) -> String
where I: IntoIterator, I::Item: Borrow<(K, V)>, K: AsRef<str>, V: AsRef<str> {
    serialize_internal(pairs, EncodingOverride::utf8())
}

/// Convert an iterator of (name, value) pairs
/// into a string in the `application/x-www-form-urlencoded` format.
///
/// This function is only available if the `query_encoding` Cargo feature is enabled.
///
/// Arguments:
///
/// * `encoding_override`: The character encoding each name and values is encoded as
///    before percent-encoding. Defaults to UTF-8.
#[cfg(feature = "query_encoding")]
#[inline]
pub fn serialize_with_encoding<I, K, V>(pairs: I,
                                        encoding_override: Option<::encoding::EncodingRef>)
                                        -> String
where I: IntoIterator, I::Item: Borrow<(K, V)>, K: AsRef<str>, V: AsRef<str> {
    serialize_internal(pairs, EncodingOverride::from_opt_encoding(encoding_override))
}

fn serialize_internal<I, K, V>(pairs: I, encoding_override: EncodingOverride) -> String
where I: IntoIterator, I::Item: Borrow<(K, V)>, K: AsRef<str>, V: AsRef<str> {
    #[inline]
    fn byte_serialize(input: &str, output: &mut String,
                      encoding_override: EncodingOverride) {
        for &byte in encoding_override.encode(input).iter() {
            if byte == b' ' {
                output.push_str("+")
            } else {
                percent_encode_to(&[byte], FORM_URLENCODED_ENCODE_SET, output)
            }
        }
    }

    let mut output = String::new();
    for pair in pairs {
        let &(ref name, ref value) = pair.borrow();
        if output.len() > 0 {
            output.push_str("&");
        }
        byte_serialize(name.as_ref(), &mut output, encoding_override);
        output.push_str("=");
        byte_serialize(value.as_ref(), &mut output, encoding_override);
    }
    output
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_urlencoded() {
        let pairs = &[
            ("foo".to_string(), "é&".to_string()),
            ("bar".to_string(), "".to_string()),
            ("foo".to_string(), "#".to_string())
        ];
        let encoded = serialize(pairs);
        assert_eq!(encoded, "foo=%C3%A9%26&bar=&foo=%23");
        assert_eq!(parse(encoded.as_bytes()), pairs.to_vec());
    }

    #[test]
    fn test_form_serialize() {
        let pairs = [("foo", "é&"),
                     ("bar", ""),
                     ("foo", "#")];

        let want = "foo=%C3%A9%26&bar=&foo=%23";
        // Works with referenced tuples
        assert_eq!(serialize(pairs.iter()), want);
        // Works with owned tuples
        assert_eq!(serialize(pairs.iter().map(|p| (p.0, p.1))), want);

    }
}

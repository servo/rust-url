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

use std::ascii::AsciiExt;
use std::borrow::{Borrow, Cow};
use encoding::EncodingOverride;
use percent_encoding::{percent_encode, percent_decode, FORM_URLENCODED_ENCODE_SET};


/// Convert a byte string in the `application/x-www-form-urlencoded` format
/// into a iterator of (name, value) pairs.
///
/// Use `parse(input.as_bytes())` to parse a `&str` string.
///
/// The names and values are percent-decoded. For instance, `%23first=%25try%25` will be
/// converted to `[("#first", "%try%")]`.
#[inline]
pub fn parse(input: &[u8]) -> Parser {
    Parser {
        input: input,
        encoding: EncodingOverride::utf8(),
    }
}


/// Convert a byte string in the `application/x-www-form-urlencoded` format
/// into a iterator of (name, value) pairs.
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
pub fn parse_with_encoding<'a>(input: &'a [u8],
                               encoding_override: Option<::encoding::EncodingRef>,
                               use_charset: bool)
                               -> Result<Parser<'a>, ()> {
    let mut encoding = EncodingOverride::from_opt_encoding(encoding_override);
    if !(encoding.is_utf8() || input.is_ascii()) {
        return Err(())
    }
    if use_charset {
        for sequence in input.split(|&b| b == b'&') {
            // No '+' in "_charset_" to replace with ' '.
            if sequence.starts_with(b"_charset_=") {
                let value = &sequence[b"_charset_=".len()..];
                // Skip replacing '+' with ' ' in value since no encoding label contains either:
                // https://encoding.spec.whatwg.org/#names-and-labels
                if let Some(e) = EncodingOverride::lookup(value) {
                    encoding = e;
                    break
                }
            }
        }
    }
    Ok(Parser {
        input: input,
        encoding: encoding,
    })
}

/// The return type of `parse()`.
pub struct Parser<'a> {
    input: &'a [u8],
    encoding: EncodingOverride,
}

impl<'a> Iterator for Parser<'a> {
    type Item = (Cow<'a, str>, Cow<'a, str>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.input.is_empty() {
                return None
            }
            let mut split2 = self.input.splitn(2, |&b| b == b'&');
            let sequence = split2.next().unwrap();
            self.input = split2.next().unwrap_or(&[][..]);
            if sequence.is_empty() {
                continue
            }
            let mut split2 = sequence.splitn(2, |&b| b == b'=');
            let name = split2.next().unwrap();
            let value = split2.next().unwrap_or(&[][..]);
            return Some((
                decode(name, self.encoding),
                decode(value, self.encoding),
            ))
        }
    }
}

/// * Replace b'+' with b' '
/// * Then percent-decode
/// * Then decode with `encoding`
fn decode<'a>(input: &'a [u8], encoding: EncodingOverride) -> Cow<'a, str> {
    // The return value can borrow `input` but not an intermediate Cow,
    // so we need to return Owned if either of the intermediate Cow is Owned
    match replace_plus(input) {
        Cow::Owned(replaced) => {
            let decoded: Cow<_> = percent_decode(&replaced).into();
            encoding.decode(&decoded).into_owned().into()
        }
        Cow::Borrowed(replaced) => {
            match percent_decode(replaced).into() {
                Cow::Owned(decoded) => encoding.decode(&decoded).into_owned().into(),
                Cow::Borrowed(decoded) => encoding.decode(decoded),
            }
        }
    }
}

/// Replace b'+' with b' '
fn replace_plus<'a>(input: &'a [u8]) -> Cow<'a, [u8]> {
    match input.iter().position(|&b| b == b'+') {
        None => input.into(),
        Some(first_position) => {
            let mut replaced = input.to_owned();
            replaced[first_position] = b' ';
            for byte in &mut replaced[first_position + 1..] {
                if *byte == b'+' {
                    *byte = b' ';
                }
            }
            replaced.into()
        }
    }
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
    serialize_internal(pairs, EncodingOverride::from_opt_encoding(encoding_override).to_output_encoding())
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
                output.extend(percent_encode(&[byte], FORM_URLENCODED_ENCODE_SET))
            }
        }
    }

    let mut output = String::new();
    for pair in pairs {
        let &(ref name, ref value) = pair.borrow();
        if !output.is_empty() {
            output.push_str("&");
        }
        byte_serialize(name.as_ref(), &mut output, encoding_override);
        output.push_str("=");
        byte_serialize(value.as_ref(), &mut output, encoding_override);
    }
    output
}

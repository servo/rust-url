// Copyright 2013-2015 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Parser and serializer for the [`application/x-www-form-urlencoded` syntax](
//! http://url.spec.whatwg.org/#application/x-www-form-urlencoded),
//! as used by HTML forms.
//!
//! Converts between a string (such as an URL’s query string)
//! and a sequence of (name, value) pairs.

use encoding::EncodingOverride;
use percent_encoding::{percent_encode_byte, percent_decode};
use std::borrow::{Borrow, Cow};
use std::str;


/// Convert a byte string in the `application/x-www-form-urlencoded` syntax
/// into a iterator of (name, value) pairs.
///
/// Use `parse(input.as_bytes())` to parse a `&str` string.
///
/// The names and values are percent-decoded. For instance, `%23first=%25try%25` will be
/// converted to `[("#first", "%try%")]`.
#[inline]
pub fn parse(input: &[u8]) -> Parse {
    Parse {
        input: input,
        encoding: EncodingOverride::utf8(),
    }
}


/// Convert a byte string in the `application/x-www-form-urlencoded` syntax
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
                               -> Result<Parse<'a>, ()> {
    use std::ascii::AsciiExt;

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
    Ok(Parse {
        input: input,
        encoding: encoding,
    })
}

/// The return type of `parse()`.
pub struct Parse<'a> {
    input: &'a [u8],
    encoding: EncodingOverride,
}

impl<'a> Iterator for Parse<'a> {
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

fn decode(input: &[u8], encoding: EncodingOverride) -> Cow<str> {
    let replaced = replace_plus(input);
    encoding.decode(match percent_decode(&replaced).if_any() {
        Some(vec) => vec.into(),
        None => replaced,
    })
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

/// The [`application/x-www-form-urlencoded` byte serializer](
/// https://url.spec.whatwg.org/#concept-urlencoded-byte-serializer).
///
/// Return an iterator of `&str` slices.
pub fn byte_serialize(input: &[u8]) -> ByteSerialize {
    ByteSerialize {
        bytes: input,
    }
}

/// Return value of `byte_serialize()`.
pub struct ByteSerialize<'a> {
    bytes: &'a [u8],
}

fn byte_serialized_unchanged(byte: u8) -> bool {
    matches!(byte, b'*' | b'-' | b'.' | b'0' ... b'9' | b'A' ... b'Z' | b'_' | b'a' ... b'z')
}

impl<'a> Iterator for ByteSerialize<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if let Some((&first, tail)) = self.bytes.split_first() {
            if !byte_serialized_unchanged(first) {
                self.bytes = tail;
                return Some(if first == b' ' { "+" } else { percent_encode_byte(first) })
            }
            let position = tail.iter().position(|&b| !byte_serialized_unchanged(b));
            let (unchanged_slice, remaining) = match position {
                // 1 for first_byte + i unchanged in tail
                Some(i) => self.bytes.split_at(1 + i),
                None => (self.bytes, &[][..]),
            };
            self.bytes = remaining;
            Some(unsafe { str::from_utf8_unchecked(unchanged_slice) })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.bytes.is_empty() {
            (0, Some(0))
        } else {
            (1, Some(self.bytes.len()))
        }
    }
}

/// The [`application/x-www-form-urlencoded` serializer](
/// https://url.spec.whatwg.org/#concept-urlencoded-serializer).
pub struct Serializer<'a> {
    string: &'a mut String,
    start_position: usize,
    encoding: EncodingOverride,
}

impl<'a> Serializer<'a> {
    /// Create a new `application/x-www-form-urlencoded` serializer
    /// for the given range of the given string.
    ///
    /// If the range is non-empty, the corresponding slice of the string is assumed
    /// to already be in `application/x-www-form-urlencoded` syntax.
    pub fn new(string: &'a mut String, start_position: usize) -> Self {
        &string[start_position..];  // Panic if out of bounds
        Serializer {
            string: string,
            start_position: start_position,
            encoding: EncodingOverride::utf8(),
        }
    }

    /// Remove any existing name/value pair.
    pub fn clear(&mut self) {
        self.string.truncate(self.start_position)
    }

    /// Set the character encoding to be used for names and values before percent-encoding.
    #[cfg(feature = "query_encoding")]
    pub fn encoding_override(&mut self, new: Option<::encoding::EncodingRef>) {
        self.encoding = EncodingOverride::from_opt_encoding(new).to_output_encoding();;
    }

    fn append_separator_if_needed(&mut self) {
        if self.string.len() > self.start_position {
            self.string.push('&')
        }
    }

    /// Serialize and append a name/value pair.
    pub fn append_pair(&mut self, name: &str, value: &str) {
        self.append_separator_if_needed();
        self.string.extend(byte_serialize(&self.encoding.encode(name.into())));
        self.string.push('=');
        self.string.extend(byte_serialize(&self.encoding.encode(value.into())));
    }

    /// Serialize and append a number of name/value pairs.
    ///
    /// This simply calls `append_pair` repeatedly.
    /// This can be more convenient, so the user doesn’t need to introduce a block
    /// to limit the scope of `Serializer`’s borrow of its string.
    pub fn append_pairs<I, K, V>(&mut self, iter: I)
    where I: IntoIterator, I::Item: Borrow<(K, V)>, K: AsRef<str>, V: AsRef<str> {
        for pair in iter {
            let &(ref k, ref v) = pair.borrow();
            self.append_pair(k.as_ref(), v.as_ref())
        }
    }

    /// Add a name/value pair whose name is `_charset_`
    /// and whose value is the character encoding’s name.
    /// (See the `encoding_override()` method.)
    #[cfg(feature = "query_encoding")]
    pub fn append_charset(&mut self) {
        self.append_separator_if_needed();
        self.string.push_str("_charset_=");
        self.string.push_str(self.encoding.name());
    }
}

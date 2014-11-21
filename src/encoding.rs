// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! Abstraction that conditionally compiles either to rust-encoding,
//! or to only support UTF-8.

#[cfg(feature = "query_encoding")]
extern crate encoding;

#[cfg(feature = "query_encoding")]
use self::encoding::types::{DecoderTrap, EncoderTrap};

#[cfg(feature = "query_encoding")]
use self::encoding::label::encoding_from_whatwg_label;

#[cfg(feature = "query_encoding")]
pub use self::encoding::types::EncodingRef;


#[cfg(feature = "query_encoding")]
pub struct EncodingOverride {
    /// `None` means UTF-8.
    encoding: Option<EncodingRef>
}

#[cfg(feature = "query_encoding")]
impl EncodingOverride {
    pub fn from_opt_encoding(encoding: Option<EncodingRef>) -> EncodingOverride {
        encoding.map(EncodingOverride::from_encoding).unwrap_or_else(EncodingOverride::utf8)
    }

    pub fn from_encoding(encoding: EncodingRef) -> EncodingOverride {
        EncodingOverride {
            encoding: if encoding.name() == "utf-8" { None } else { Some(encoding) }
        }
    }

    pub fn utf8() -> EncodingOverride {
        EncodingOverride { encoding: None }
    }

    pub fn lookup(label: &[u8]) -> Option<EncodingOverride> {
        ::std::str::from_utf8(label.as_slice())
        .and_then(encoding_from_whatwg_label)
        .map(EncodingOverride::from_encoding)
    }

    pub fn is_utf8(&self) -> bool {
        self.encoding.is_none()
    }

    pub fn decode(&self, input: &[u8]) -> String {
        match self.encoding {
            Some(encoding) => encoding.decode(input, DecoderTrap::Replace).unwrap(),
            None => String::from_utf8_lossy(input).into_string(),
        }
    }

    // For UTF-8, we want to return the &[u8] bytes of the &str input strings without copying
    // But for other encodings we have to allocate a new Vec<u8>.
    // To return &[u8] in that case, the vector has to be kept somewhere
    // that lives at least as long as the return value.
    // Therefore, the caller provides a temporary Vec<u8> as scratch space.
    //
    // FIXME: Return std::borrow::Cow<'a, Vec<u8>, [u8]> instead.
    pub fn encode<'a>(&self, input: &'a str, tmp: &'a mut Vec<u8>) -> &'a [u8] {
        match self.encoding {
            Some(encoding) => {
                *tmp = encoding.encode(input.as_slice(), EncoderTrap::NcrEscape).unwrap();
                tmp.as_slice()
            },
            None => input.as_bytes()  // UTF-8
        }
    }
}


#[cfg(not(feature = "query_encoding"))]
pub struct EncodingOverride;

#[cfg(not(feature = "query_encoding"))]
impl EncodingOverride {
    pub fn utf8() -> EncodingOverride {
        EncodingOverride
    }

    pub fn lookup(_label: &[u8]) -> Option<EncodingOverride> {
        None
    }

    pub fn is_utf8(&self) -> bool {
        true
    }

    pub fn decode(&self, input: &[u8]) -> String {
        String::from_utf8_lossy(input).into_string()
    }

    pub fn encode<'a>(&self, input: &'a str, _: &'a mut Vec<u8>) -> &'a [u8] {
        input.as_bytes()
    }
}

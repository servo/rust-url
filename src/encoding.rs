// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! Abstraction that conditionally compiles either to rust-encoding,
//! or to only support UTF-8.

#[cfg(feature = "query_encoding")] extern crate encoding;

use std::borrow::Cow;

#[cfg(feature = "query_encoding")] use self::encoding::types::{DecoderTrap, EncoderTrap};
#[cfg(feature = "query_encoding")] use self::encoding::label::encoding_from_whatwg_label;
#[cfg(feature = "query_encoding")] pub use self::encoding::types::EncodingRef;

#[cfg(feature = "query_encoding")]
#[derive(Copy, Clone)]
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
        ::std::str::from_utf8(label)
        .ok()
        .and_then(encoding_from_whatwg_label)
        .map(EncodingOverride::from_encoding)
    }

    pub fn is_utf8(&self) -> bool {
        self.encoding.is_none()
    }

    pub fn decode(&self, input: &[u8]) -> String {
        match self.encoding {
            Some(encoding) => encoding.decode(input, DecoderTrap::Replace).unwrap(),
            None => String::from_utf8_lossy(input).to_string(),
        }
    }

    pub fn encode<'a>(&self, input: &'a str) -> Cow<'a, [u8]> {
        match self.encoding {
            Some(encoding) => Cow::Owned(
                encoding.encode(input, EncoderTrap::NcrEscape).unwrap()),
            None => Cow::Borrowed(input.as_bytes()),  // UTF-8
        }
    }
}


#[cfg(not(feature = "query_encoding"))]
#[derive(Copy, Clone)]
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
        String::from_utf8_lossy(input).into_owned()
    }

    pub fn encode<'a>(&self, input: &'a str) -> Cow<'a, [u8]> {
        Cow::Borrowed(input.as_bytes())
    }
}

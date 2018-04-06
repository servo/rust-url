// Copyright 2013-2018 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! Implementation using rust-encoding (legacy).
//! Only built with feature flag `query_encoding`.

extern crate encoding;

use encoding::EncodingOverride;
use encoding::utf8_helpers::{decode_utf8_lossy, encode_utf8};

use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};

use self::encoding::types::{DecoderTrap, EncoderTrap};
use self::encoding::label::encoding_from_whatwg_label;
pub use self::encoding::types::EncodingRef;

#[derive(Copy, Clone)]
pub struct EncodingOverrideLegacy {
    /// `None` means UTF-8.
    encoding: Option<EncodingRef>
}

impl EncodingOverrideLegacy {
    pub fn from_opt_encoding(encoding: Option<EncodingRef>) -> Self {
        encoding.map(Self::from_encoding).unwrap_or_else(Self::utf8)
    }

    pub fn from_encoding(encoding: EncodingRef) -> Self {
        Self {
            encoding: if encoding.name() == "utf-8" { None } else { Some(encoding) }
        }
    }
}

impl EncodingOverride for EncodingOverrideLegacy {
    #[inline]
    fn utf8() -> Self {
        Self { encoding: None }
    }

    fn lookup(label: &[u8]) -> Option<Self> {
        // Don't use String::from_utf8_lossy since no encoding label contains U+FFFD
        // https://encoding.spec.whatwg.org/#names-and-labels
        ::std::str::from_utf8(label)
        .ok()
        .and_then(encoding_from_whatwg_label)
        .map(Self::from_encoding)
    }

    fn is_utf8(&self) -> bool {
        self.encoding.is_none()
    }

    fn name(&self) -> &'static str {
        match self.encoding {
            Some(encoding) => encoding.name(),
            None => "utf-8",
        }
    }

    fn decode<'a>(&self, input: Cow<'a, [u8]>) -> Cow<'a, str> {
        match self.encoding {
            // `encoding.decode` never returns `Err` when called with `DecoderTrap::Replace`
            Some(encoding) => encoding.decode(&input, DecoderTrap::Replace).unwrap().into(),
            None => decode_utf8_lossy(input),
        }
    }

    fn encode<'a>(&self, input: Cow<'a, str>) -> Cow<'a, [u8]> {
        match self.encoding {
            // `encoding.encode` never returns `Err` when called with `EncoderTrap::NcrEscape`
            Some(encoding) => Cow::Owned(encoding.encode(&input, EncoderTrap::NcrEscape).unwrap()),
            None => encode_utf8(input)
        }
    }
}

impl Debug for EncodingOverrideLegacy {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "EncodingOverride {{ encoding: ")?;
        match self.encoding {
            Some(e) => write!(f, "{} }}", e.name()),
            None => write!(f, "None }}")
        }
    }
}

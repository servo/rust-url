// Copyright 2013-2018 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! Implementation using UTF-8 only.
//! Used when building without any query encoding feature flags.

use std::borrow::Cow;

use encoding::EncodingOverride;
use encoding::utf8_helpers::{decode_utf8_lossy, encode_utf8};

#[derive(Copy, Clone, Debug)]
pub struct EncodingOverrideFallback;

impl EncodingOverrideFallback {
    #[inline]
    pub fn utf8() -> Self {
        EncodingOverrideFallback
    }
}

impl EncodingOverride for EncodingOverrideFallback {
    fn utf8() -> Self {
        Self {}
    }

    fn lookup(_label: &[u8]) -> Option<Self> {
        // always return `None` which means UTF-8
        None
    }

    fn is_utf8(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "utf-8"
    }

    fn decode<'a>(&self, input: Cow<'a, [u8]>) -> Cow<'a, str> {
        decode_utf8_lossy(input)
    }

    fn encode<'a>(&self, input: Cow<'a, str>) -> Cow<'a, [u8]> {
        encode_utf8(input)
    }
}

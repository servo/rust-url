// Copyright 2013-2018 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! Abstraction that conditionally compiles either to encoding_rs,
//! or rust-encoding (legacy), or to only support UTF-8.

mod utf8_helpers;

use std::borrow::Cow;
use std::fmt::Debug;

#[cfg(feature = "query_encoding_2")] mod encoding_rs;
#[cfg(feature = "query_encoding_2")] use self::encoding_rs::EncodingOverrideRs;

#[cfg(feature = "query_encoding")] mod legacy;
#[cfg(feature = "query_encoding")] pub use self::legacy::{EncodingOverrideLegacy, EncodingRef};

#[cfg(not(any(feature = "query_encoding", feature = "query_encoding_2")))]
mod fallback;
#[cfg(not(any(feature = "query_encoding", feature = "query_encoding_2")))]
use self::fallback::EncodingOverrideFallback;


pub trait EncodingOverride : Debug {
    /// Get an Encoding representing UTF-8.
    fn utf8() -> Self where Self: Sized;

    /// Look up an Encoding using the WHATWG label,
    /// listed at https://encoding.spec.whatwg.org/#names-and-labels
    fn lookup(label: &[u8]) -> Option<Self> where Self: Sized;

    /// Whether this Encoding represents UTF-8.
    fn is_utf8(&self) -> bool;

    /// Get the name of this Encoding, which when ASCII lowercased, may be used as a
    /// lookup label. https://encoding.spec.whatwg.org/#names-and-labels
    fn name(&self) -> &'static str;

    /// https://encoding.spec.whatwg.org/#get-an-output-encoding
    fn to_output_encoding(self) -> Self where Self: Sized {
        if !self.is_utf8() {
            let lowercased = self.name().to_lowercase();
            if lowercased == "utf-16le" || lowercased == "utf-16be" {
                return Self::utf8()
            }
        }
        self
    }

    /// Decode the specified bytes in the current encoding, to UTF-8.
    fn decode<'a>(&self, input: Cow<'a, [u8]>) -> Cow<'a, str>;

    /// Encode the UTF-8 string to the current encoding.
    fn encode<'a>(&self, input: Cow<'a, str>) -> Cow<'a, [u8]>;
}

#[cfg(feature = "query_encoding_2")]
pub fn default_encoding_override() -> EncodingOverrideRs {
    EncodingOverrideRs::utf8()
}

#[cfg(all(feature = "query_encoding", not(feature = "query_encoding_2")))]
pub fn default_encoding_override() -> EncodingOverrideLegacy {
    EncodingOverrideLegacy::utf8()
}

#[cfg(not(any(feature = "query_encoding", feature = "query_encoding_2")))]
pub fn default_encoding_override() -> EncodingOverrideFallback {
    EncodingOverrideFallback::utf8()
}

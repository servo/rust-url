// Copyright 2019 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::borrow::Cow;

pub type EncodingOverride<'a> = Option<&'a dyn Fn(&str) -> Cow<[u8]>>;

pub(crate) fn encode<'a>(encoding_override: EncodingOverride, input: &'a str) -> Cow<'a, [u8]> {
    if let Some(o) = encoding_override {
        return o(input);
    }
    input.as_bytes().into()
}

pub(crate) fn decode_utf8_lossy(input: Cow<[u8]>) -> Cow<str> {
    // Note: This function is duplicated in `percent_encoding/lib.rs`.
    match input {
        Cow::Borrowed(bytes) => String::from_utf8_lossy(bytes),
        Cow::Owned(bytes) => {
            match String::from_utf8_lossy(&bytes) {
                Cow::Borrowed(utf8) => {
                    // If from_utf8_lossy returns a Cow::Borrowed, then we can
                    // be sure our original bytes were valid UTF-8. This is because
                    // if the bytes were invalid UTF-8 from_utf8_lossy would have
                    // to allocate a new owned string to back the Cow so it could
                    // replace invalid bytes with a placeholder.

                    // First we do a debug_assert to confirm our description above.
                    let raw_utf8: *const [u8];
                    raw_utf8 = utf8.as_bytes();
                    debug_assert!(raw_utf8 == &*bytes as *const [u8]);

                    // Given we know the original input bytes are valid UTF-8,
                    // and we have ownership of those bytes, we re-use them and
                    // return a Cow::Owned here. Ideally we'd put our return statement
                    // right below this line, but to support the old lexically scoped
                    // borrow checker the return must be moved to outside the match
                    // statement.
                    Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) })
                }
                Cow::Owned(s) => Cow::Owned(s),
            }
        }
    }
}

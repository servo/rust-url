// Copyright 2013-2018 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! Implementation using [encoding_rs](https://github.com/hsivonen/encoding_rs).
//! Only built with feature flag `query_encoding_2`.

extern crate encoding_rs;

use encoding::EncodingOverride;
use encoding::utf8_helpers::{decode_utf8_lossy, encode_utf8};

use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};

use self::encoding_rs::Encoding;

pub struct EncodingOverrideRs {
    /// `None` means UTF-8.
    encoding: Option<&'static Encoding>
}

impl EncodingOverrideRs {
    fn from_encoding(encoding: &'static Encoding) -> Self {
        Self {
            encoding: if encoding.name() == "UTF-8" { None } else { Some(encoding) }
        }
    }
}

impl EncodingOverride for EncodingOverrideRs {
    #[inline]
    fn utf8() -> Self {
        Self { encoding: None }
    }

    fn lookup(label: &[u8]) -> Option<Self> {
        // Don't use String::from_utf8_lossy since no encoding label contains U+FFFD
        // https://encoding.spec.whatwg.org/#names-and-labels
        Encoding::for_label(label)
            .map(Self::from_encoding)
    }

    fn is_utf8(&self) -> bool {
        self.encoding.is_none()
    }

    fn name(&self) -> &'static str {
        match self.encoding {
            Some(encoding) => encoding.name(),
            None => encoding_rs::UTF_8.name(),
        }
    }

    fn decode<'a>(&self, input: Cow<'a, [u8]>) -> Cow<'a, str> {
        match self.encoding {
            Some(encoding) => {
                match input {
                    Cow::Borrowed(b) => {
                        let (cow, _) = encoding.decode_without_bom_handling(b);
                        cow
                    },
                    Cow::Owned(v) => {
                        {
                            let (cow, _) = encoding.decode_without_bom_handling(&v[..]);
                            match cow {
                                Cow::Owned(s) => {
                                    // Free old heap buffer and return a new one.
                                    return Cow::Owned(s);
                                },
                                Cow::Borrowed(_) => {},
                            }
                        }
                        // Reuse the old heap buffer.
                        Cow::Owned(unsafe { String::from_utf8_unchecked(v) })
                    },
                }
            },
            None => decode_utf8_lossy(input),
        }
    }

    fn encode<'a>(&self, input: Cow<'a, str>) -> Cow<'a, [u8]> {
        match self.encoding {
            Some(encoding) => {
                match input {
                    Cow::Borrowed(s) => {
                        let (cow, _, _) = encoding.encode(s);
                        cow
                    },
                    Cow::Owned(s) => {
                        {
                            let (cow, _, _) = encoding.encode(&s[..]);
                            match cow {
                                Cow::Owned(v) => {
                                    // Free old heap buffer and return a new one.
                                    return Cow::Owned(v);
                                },
                                Cow::Borrowed(_) => {},
                            }
                        }
                        // Reuse the old heap buffer.
                        Cow::Owned(s.into_bytes())
                    },
                }
            },
            None => encode_utf8(input),
        }
    }
}

impl Debug for EncodingOverrideRs {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "EncodingOverride {{ encoding: ")?;
        match self.encoding {
            Some(e) => write!(f, "{} }}", e.name()),
            None => write!(f, "None }}")
        }
    }
}

// Copyright 2016 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This Rust crate implements IDNA
//! [per the WHATWG URL Standard](https://url.spec.whatwg.org/#idna).
//!
//! It also exposes the underlying algorithms from [*Unicode IDNA Compatibility Processing*
//! (Unicode Technical Standard #46)](http://www.unicode.org/reports/tr46/)
//! and [Punycode (RFC 3492)](https://tools.ietf.org/html/rfc3492).
//!
//! Quoting from [UTS #46’s introduction](http://www.unicode.org/reports/tr46/#Introduction):
//!
//! > Initially, domain names were restricted to ASCII characters.
//! > A system was introduced in 2003 for internationalized domain names (IDN).
//! > This system is called Internationalizing Domain Names for Applications,
//! > or IDNA2003 for short.
//! > This mechanism supports IDNs by means of a client software transformation
//! > into a format known as Punycode.
//! > A revision of IDNA was approved in 2010 (IDNA2008).
//! > This revision has a number of incompatibilities with IDNA2003.
//! >
//! > The incompatibilities force implementers of client software,
//! > such as browsers and emailers,
//! > to face difficult choices during the transition period
//! > as registries shift from IDNA2003 to IDNA2008.
//! > This document specifies a mechanism
//! > that minimizes the impact of this transition for client software,
//! > allowing client software to access domains that are valid under either system.
#![no_std]

// For forwards compatibility
#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

#[cfg(not(feature = "alloc"))]
compile_error!("the `alloc` feature must be enabled");

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

use alloc::borrow::Cow;
use alloc::string::String;
use uts46bis::Uts46;

pub mod punycode;
mod deprecated;
pub mod uts46bis;

#[allow(deprecated)]
pub use crate::deprecated::{Config, Idna};

/// Type indicating that there were errors during UTS #46 processing.
#[derive(Default, Debug)]
#[non_exhaustive]
pub struct Errors {}

impl From<Errors> for Result<(), Errors> {
    fn from(e: Errors) -> Result<(), Errors> {
        Err(e)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Errors {}

impl core::fmt::Display for Errors {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

/// The [domain to ASCII](https://url.spec.whatwg.org/#concept-domain-to-ascii) algorithm;
/// version returning a `Cow`.
///
/// Return the ASCII representation a domain name,
/// normalizing characters (upper-case to lower-case and other kinds of equivalence)
/// and using Punycode as necessary.
///
/// This process may fail.
pub fn domain_to_ascii_cow<'a>(domain: &'a str) -> Result<Cow<'a, str>, Errors> {
    Uts46::new().to_ascii(domain.as_bytes(), uts46bis::Strictness::WhatwgUserAgent)
}

/// The [domain to ASCII](https://url.spec.whatwg.org/#concept-domain-to-ascii) algorithm;
/// version returning `String`. See also [`domain_to_ascii_cow`].
///
/// Return the ASCII representation a domain name,
/// normalizing characters (upper-case to lower-case and other kinds of equivalence)
/// and using Punycode as necessary.
///
/// This process may fail.
pub fn domain_to_ascii(domain: &str) -> Result<String, Errors> {
    domain_to_ascii_cow(domain).map(|cow| cow.into_owned())
}

/// The [domain to ASCII](https://url.spec.whatwg.org/#concept-domain-to-ascii) algorithm,
/// with the `beStrict` flag set.
///
/// Note that this rejects various real-world names including:
/// * YouTube CDN nodes
/// * Some GitHub user pages
/// * Pseudo-hosts used by various TXT record-based protocols.
pub fn domain_to_ascii_strict(domain: &str) -> Result<String, Errors> {
    Uts46::new()
        .to_ascii(
            domain.as_bytes(),
            uts46bis::Strictness::Std3ConformanceChecker,
        )
        .map(|cow| cow.into_owned())
}

/// The [domain to Unicode](https://url.spec.whatwg.org/#concept-domain-to-unicode) algorithm;
/// version returning a `Cow`.
///
/// Return the Unicode representation of a domain name,
/// normalizing characters (upper-case to lower-case and other kinds of equivalence)
/// and decoding Punycode as necessary.
///
/// If the second item of the tuple indicates an error, the first item of the tuple
/// denotes errors using the REPLACEMENT CHARACTERs in order to be able to illustrate
/// errors to the user. When the second item of the return tuple signals an error,
/// the first item of the tuple must not be used in a network protocol.
pub fn domain_to_unicode_cow<'a>(domain: &'a str) -> (Cow<'a, str>, Result<(), Errors>) {
    Uts46::new().to_unicode(domain.as_bytes(), uts46bis::Strictness::WhatwgUserAgent)
}

/// The [domain to Unicode](https://url.spec.whatwg.org/#concept-domain-to-unicode) algorithm;
/// version returning `String`. See also [`domain_to_unicode_cow`].
///
/// Return the Unicode representation of a domain name,
/// normalizing characters (upper-case to lower-case and other kinds of equivalence)
/// and decoding Punycode as necessary.
///
/// If the second item of the tuple indicates an error, the first item of the tuple
/// denotes errors using the REPLACEMENT CHARACTERs in order to be able to illustrate
/// errors to the user. When the second item of the return tuple signals an error,
/// the first item of the tuple must not be used in a network protocol.
pub fn domain_to_unicode(domain: &str) -> (String, Result<(), Errors>) {
    let (cow, result) = domain_to_unicode_cow(domain);
    (cow.into_owned(), result)
}

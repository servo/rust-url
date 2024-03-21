// Copyright 2013-2014 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [*Unicode IDNA Compatibility Processing*
//! (Unicode Technical Standard #46)](http://www.unicode.org/reports/tr46/)

#![allow(deprecated)]

use alloc::string::String;

use crate::uts46::*;
use crate::Errors;

/// Deprecated. Use the crate-top-level functions or [`Uts46`].
#[derive(Default)]
#[deprecated]
pub struct Idna {
    config: Config,
}

impl Idna {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// [UTS 46 ToASCII](http://www.unicode.org/reports/tr46/#ToASCII)
    #[allow(clippy::wrong_self_convention)]
    pub fn to_ascii(&mut self, domain: &str, out: &mut String) -> Result<(), Errors> {
        match Uts46::new().process(
            domain.as_bytes(),
            self.config.strictness(),
            ErrorPolicy::FailFast,
            |_, _, _| false,
            out,
            None,
        ) {
            Ok(ProcessingSuccess::Passthrough) => {
                if self.config.verify_dns_length && !verify_dns_length(domain) {
                    return Err(crate::Errors::default());
                }
                out.push_str(domain);
                Ok(())
            }
            Ok(ProcessingSuccess::WroteToSink) => {
                if self.config.verify_dns_length && !verify_dns_length(out) {
                    return Err(crate::Errors::default());
                }
                Ok(())
            }
            Err(ProcessingError::ValidityError) => Err(crate::Errors::default()),
            Err(ProcessingError::SinkError) => unreachable!(),
        }
    }

    /// [UTS 46 ToUnicode](http://www.unicode.org/reports/tr46/#ToUnicode)
    #[allow(clippy::wrong_self_convention)]
    pub fn to_unicode(&mut self, domain: &str, out: &mut String) -> Result<(), Errors> {
        match Uts46::new().process(
            domain.as_bytes(),
            self.config.strictness(),
            ErrorPolicy::MarkErrors,
            |_, _, _| true,
            out,
            None,
        ) {
            Ok(ProcessingSuccess::Passthrough) => {
                out.push_str(domain);
                Ok(())
            }
            Ok(ProcessingSuccess::WroteToSink) => Ok(()),
            Err(ProcessingError::ValidityError) => Err(crate::Errors::default()),
            Err(ProcessingError::SinkError) => unreachable!(),
        }
    }
}

/// Deprecated configuration API.
#[derive(Clone, Copy)]
#[must_use]
#[deprecated]
pub struct Config {
    use_std3_ascii_rules: bool,
    verify_dns_length: bool,
    check_hyphens: bool,
}

/// The defaults are that of _beStrict=false_ in the [WHATWG URL Standard](https://url.spec.whatwg.org/#idna)
impl Default for Config {
    fn default() -> Self {
        Config {
            use_std3_ascii_rules: false,
            check_hyphens: false,
            // Only use for to_ascii, not to_unicode
            verify_dns_length: false,
        }
    }
}

impl Config {
    /// Whether to enforce STD3 or WHATWG URL Standard ASCII deny list.
    ///
    /// `true` for STD3, `false` for WHATWG.
    ///
    /// Note that `true` rejects pseudo-hosts used by various TXT record-based protocols.
    ///
    /// Must be set to the same value as [`Config::check_hyphens`].
    #[inline]
    pub fn use_std3_ascii_rules(mut self, value: bool) -> Self {
        self.use_std3_ascii_rules = value;
        self
    }

    /// Obsolete method retained to ease migration. The argument must be `false`.
    ///
    /// Panics
    ///
    /// If the argument is `true`.
    #[inline]
    #[allow(unused_mut)]
    pub fn transitional_processing(mut self, value: bool) -> Self {
        assert!(!value, "Transitional processing is no longer supported");
        self
    }

    /// Whether the _VerifyDNSLength_ operation should be performed
    /// by `to_ascii`.
    #[inline]
    pub fn verify_dns_length(mut self, value: bool) -> Self {
        self.verify_dns_length = value;
        self
    }

    /// Whether to enforce IETF rules for hyphen placement.
    ///
    /// `true` to deny hyphens in the first, last, third, and fourth
    /// position of a label. `false` to not enforce.
    ///
    /// Note that `true` rejects real-world names, including YouTube CDN nodes
    /// and some GitHub user pages.
    ///
    /// Must be set to the same value as [`Config::use_std3_ascii_rules`].
    #[inline]
    pub fn check_hyphens(mut self, value: bool) -> Self {
        self.check_hyphens = value;
        self
    }

    /// Obsolete method retained to ease migration. The argument must be `false`.
    ///
    /// Panics
    ///
    /// If the argument is `true`.
    #[inline]
    #[allow(unused_mut)]
    pub fn use_idna_2008_rules(mut self, value: bool) -> Self {
        assert!(!value, "IDNA 2008 rules are no longer supported");
        self
    }

    /// Compute strictness
    fn strictness(&self) -> Strictness {
        assert_eq!(self.check_hyphens, self.use_std3_ascii_rules, "Setting check_hyphens and use_std3_ascii_rules to different values is no longer supported");
        if self.use_std3_ascii_rules {
            Strictness::Std3ConformanceChecker
        } else {
            Strictness::WhatwgUserAgent
        }
    }

    /// [UTS 46 ToASCII](http://www.unicode.org/reports/tr46/#ToASCII)
    pub fn to_ascii(self, domain: &str) -> Result<String, Errors> {
        let mut result = String::with_capacity(domain.len());
        let mut codec = Idna::new(self);
        codec.to_ascii(domain, &mut result).map(|()| result)
    }

    /// [UTS 46 ToUnicode](http://www.unicode.org/reports/tr46/#ToUnicode)
    pub fn to_unicode(self, domain: &str) -> (String, Result<(), Errors>) {
        let mut codec = Idna::new(self);
        let mut out = String::with_capacity(domain.len());
        let result = codec.to_unicode(domain, &mut out);
        (out, result)
    }
}

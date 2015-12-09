//! International domain names
//!
//! https://url.spec.whatwg.org/#idna

use self::Mapping::*;
use parser::ParseError;
use punycode;
use std::ascii::AsciiExt;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use unicode_bidi::{BidiClass, bidi_class};

include!("idna_mapping.rs");

#[derive(Debug)]
enum Mapping {
    Valid,
    Ignored,
    Mapped(&'static str),
    Deviation(&'static str),
    Disallowed,
    DisallowedStd3Valid,
    DisallowedStd3Mapped(&'static str),
}

struct Range {
    pub from: char,
    pub to: char,
    pub mapping: Mapping,
}

fn find_char(codepoint: char) -> &'static Mapping {
    let mut min = 0;
    let mut max = TABLE.len() - 1;
    while max > min {
        let mid = (min + max) >> 1;
        if codepoint > TABLE[mid].to {
           min = mid;
        } else if codepoint < TABLE[mid].from {
            max = mid;
        } else {
            min = mid;
            max = mid;
        }
    }
    &TABLE[min].mapping
}

fn map_char(codepoint: char, flags: Uts46Flags, output: &mut String, errors: &mut Vec<Error>) {
    match *find_char(codepoint) {
        Mapping::Valid => output.push(codepoint),
        Mapping::Ignored => {},
        Mapping::Mapped(mapping) => output.push_str(mapping),
        Mapping::Deviation(mapping) => {
            if flags.transitional_processing {
                output.push_str(mapping)
            } else {
                output.push(codepoint)
            }
        }
        Mapping::Disallowed => {
            errors.push(Error::DissallowedCharacter);
            output.push(codepoint);
        }
        Mapping::DisallowedStd3Valid => {
            if flags.use_std3_ascii_rules {
                errors.push(Error::DissallowedByStd3AsciiRules);
            }
            output.push(codepoint)
        }
        Mapping::DisallowedStd3Mapped(mapping) => {
            if flags.use_std3_ascii_rules {
                errors.push(Error::DissallowedMappedInStd3);
            }
            output.push_str(mapping)
        }
    }
}

// http://tools.ietf.org/html/rfc5893#section-2
fn passes_bidi(label: &str, transitional_processing: bool) -> bool {
    let mut chars = label.chars();
    let class = match chars.next() {
        Some(c) => bidi_class(c),
        None => return true, // empty string
    };

    if class == BidiClass::L
       || (class == BidiClass::ON && transitional_processing) // starts with \u200D
       || (class == BidiClass::ES && transitional_processing) // hack: 1.35.+33.49
       || class == BidiClass::EN // hack: starts with number 0à.\u05D0
    { // LTR
        // Rule 5
        loop {
            match chars.next() {
                Some(c) => {
                    let c = bidi_class(c);
                    if !matches!(c, BidiClass::L | BidiClass::EN |
                                    BidiClass::ES | BidiClass::CS |
                                    BidiClass::ET | BidiClass::ON |
                                    BidiClass::BN | BidiClass::NSM) {
                        return false;
                    }
                },
                None => { break; },
            }
        }

        // Rule 6
        let mut rev_chars = label.chars().rev();
        let mut last = rev_chars.next();
        loop { // must end in L or EN followed by 0 or more NSM
            match last {
                Some(c) if bidi_class(c) == BidiClass::NSM => {
                    last = rev_chars.next();
                    continue;
                }
                _ => { break; },
            }
        }

        // TODO: does not pass for àˇ.\u05D0
        // match last {
        //     Some(c) if bidi_class(c) == BidiClass::L
        //             || bidi_class(c) == BidiClass::EN => {},
        //     Some(c) => { return false; },
        //     _ => {}
        // }

    } else if class == BidiClass::R || class == BidiClass::AL { // RTL
        let mut found_en = false;
        let mut found_an = false;

        // Rule 2
        loop {
            match chars.next() {
                Some(c) => {
                    let char_class = bidi_class(c);

                    if char_class == BidiClass::EN {
                        found_en = true;
                    }
                    if char_class == BidiClass::AN {
                        found_an = true;
                    }

                    if !matches!(char_class, BidiClass::R | BidiClass::AL |
                                             BidiClass::AN | BidiClass::EN |
                                             BidiClass::ES | BidiClass::CS |
                                             BidiClass::ET | BidiClass::ON |
                                             BidiClass::BN | BidiClass::NSM) {
                        return false;
                    }
                },
                None => { break; },
            }
        }
        // Rule 3
        let mut rev_chars = label.chars().rev();
        let mut last = rev_chars.next();
        loop { // must end in L or EN followed by 0 or more NSM
            match last {
                Some(c) if bidi_class(c) == BidiClass::NSM => {
                    last = rev_chars.next();
                    continue;
                }
                _ => { break; },
            }
        }
        match last {
            Some(c) if matches!(bidi_class(c), BidiClass::R | BidiClass::AL |
                                               BidiClass::EN | BidiClass::AN) => {},
            _ => { return false; }
        }

        // Rule 4
        if found_an && found_en {
            return false;
        }
    } else {
        // Rule 2: Should start with L or R/AL
        return false;
    }

    return true;
}

/// http://www.unicode.org/reports/tr46/#Validity_Criteria
fn validate(label: &str, flags: Uts46Flags, errors: &mut Vec<Error>) {
    if label.nfc().ne(label.chars()) {
        errors.push(Error::ValidityCriteria);
    }

    // Can not contain '.' since the input is from .split('.')
    if {
        let mut chars = label.chars().skip(2);
        let third = chars.next();
        let fourth = chars.next();
        (third, fourth) == (Some('-'), Some('-'))
    } || label.starts_with("-")
        || label.ends_with("-")
        || label.chars().next().map_or(false, is_combining_mark)
        || label.chars().any(|c| match *find_char(c) {
            Mapping::Valid => false,
            Mapping::Deviation(_) => flags.transitional_processing,
            Mapping::DisallowedStd3Valid => flags.use_std3_ascii_rules,
            _ => true,
        })
        || !passes_bidi(label, flags.transitional_processing)
    {
        errors.push(Error::ValidityCriteria)
    }
}

/// http://www.unicode.org/reports/tr46/#Processing
fn uts46_processing(domain: &str, flags: Uts46Flags, errors: &mut Vec<Error>) -> String {
    let mut mapped = String::new();
    for c in domain.chars() {
        map_char(c, flags, &mut mapped, errors)
    }
    let normalized: String = mapped.nfc().collect();
    let mut validated = String::new();
    for label in normalized.split('.') {
        if validated.len() > 0 {
            validated.push('.');
        }
        if label.starts_with("xn--") {
            match punycode::decode_to_string(&label["xn--".len()..]) {
                Some(decoded_label) => {
                    let flags = Uts46Flags { transitional_processing: false, ..flags };
                    validate(&decoded_label, flags, errors);
                    validated.push_str(&decoded_label)
                }
                None => errors.push(Error::PunycodeError)
            }
        } else {
            validate(label, flags, errors);
            validated.push_str(label)
        }
    }
    validated
}

#[derive(Copy, Clone)]
pub struct Uts46Flags {
   pub use_std3_ascii_rules: bool,
   pub transitional_processing: bool,
   pub verify_dns_length: bool,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Error {
    PunycodeError,
    ValidityCriteria,
    DissallowedByStd3AsciiRules,
    DissallowedMappedInStd3,
    DissallowedCharacter,
    TooLongForDns,
}

impl From<Vec<Error>> for ParseError {
    fn from(_: Vec<Error>) -> ParseError { ParseError::IdnaError }
}

/// http://www.unicode.org/reports/tr46/#ToASCII
pub fn uts46_to_ascii(domain: &str, flags: Uts46Flags) -> Result<String, Vec<Error>> {
    let mut errors = Vec::new();
    let mut result = String::new();
    for label in uts46_processing(domain, flags, &mut errors).split('.') {
        if result.len() > 0 {
            result.push('.');
        }
        if label.is_ascii() {
            result.push_str(label);
        } else {
            match punycode::encode_str(label) {
                Some(x) => {
                    result.push_str("xn--");
                    result.push_str(&x);
                },
                None => errors.push(Error::PunycodeError)
            }
        }
    }

    if flags.verify_dns_length {
        let domain = if result.ends_with(".") { &result[..result.len()-1]  } else { &*result };
        if domain.len() < 1 || domain.len() > 253 ||
                domain.split('.').any(|label| label.len() < 1 || label.len() > 63) {
            errors.push(Error::TooLongForDns)
        }
    }
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(errors)
    }
}

/// https://url.spec.whatwg.org/#concept-domain-to-ascii
pub fn domain_to_ascii(domain: &str) -> Result<String, Vec<Error>> {
    uts46_to_ascii(domain, Uts46Flags {
        use_std3_ascii_rules: false,
        transitional_processing: true, // XXX: switch when Firefox does
        verify_dns_length: false,
    })
}

/// http://www.unicode.org/reports/tr46/#ToUnicode
///
/// Only `use_std3_ascii_rules` is used in `flags`.
pub fn uts46_to_unicode(domain: &str, mut flags: Uts46Flags) -> (String, Vec<Error>) {
    flags.transitional_processing = false;
    let mut errors = Vec::new();
    let domain = uts46_processing(domain, flags, &mut errors);
    (domain, errors)
}

/// https://url.spec.whatwg.org/#concept-domain-to-unicode
pub fn domain_to_unicode(domain: &str) -> (String, Vec<Error>) {
    uts46_to_unicode(domain, Uts46Flags {
        use_std3_ascii_rules: false,

        // Unused:
        transitional_processing: true,
        verify_dns_length: false,
    })
}

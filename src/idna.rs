//! International domain names
//!
//! https://url.spec.whatwg.org/#idna

use idna_mapping::TABLE;
use punycode;
use std::ascii::AsciiExt;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug)]
pub enum Mapping {
    Valid,
    Ignored,
    Mapped(&'static str),
    Deviation(&'static str),
    Disallowed,
    DisallowedStd3Valid,
    DisallowedStd3Mapped(&'static str),
}

pub struct Range {
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

fn map_char(codepoint: char, flags: Uts46Flags, output: &mut String) -> Result<(), Error> {
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
        Mapping::Disallowed => return Err(Error::DissallowedCharacter),
        Mapping::DisallowedStd3Valid => {
            if flags.use_std3_ascii_rules {
                return Err(Error::DissallowedByStd3AsciiRules);
            } else {
                output.push(codepoint)
            }
        }
        Mapping::DisallowedStd3Mapped(mapping) => {
            if flags.use_std3_ascii_rules {
                return Err(Error::DissallowedMappedInStd3);
            } else {
                output.push_str(mapping)
            }
        }
    }
    Ok(())
}

fn is_combining_mark(c: char) -> bool {
    false  // FIXME General_Category=Mark
}

/// http://www.unicode.org/reports/tr46/#Validity_Criteria
fn validate(label: &str, flags: Uts46Flags) -> Result<(), Error> {
    // Input is from nfc(), so it must be in NFC?
    // Can not contain '.' since the input is from .split('.')
    if {
        let mut chars = label.chars();
        let _first = chars.next();
        let _first = chars.next();
        let third = chars.next();
        let fourth = chars.next();
        (third, fourth) == (Some('-'), Some('-'))
    } || label.starts_with("-")
        || label.ends_with("-")
        // FIXME: are these two implied by being output of map_char()?
        || label.chars().next().map_or(false, is_combining_mark)
        || label.chars().any(|c| match *find_char(c) {
            Mapping::Valid => false,
            Mapping::Deviation(_) => flags.transitional_processing,
            Mapping::DisallowedStd3Valid => flags.use_std3_ascii_rules,
            _ => true,
        })
        // FIXME: add "The Bidi Rule" http://tools.ietf.org/html/rfc5893#section-2
    {
        Err(Error::ValidityCriteria)
    } else {
        Ok(())
    }
}

/// http://www.unicode.org/reports/tr46/#Processing
fn uts46_processing(domain: &str, flags: Uts46Flags) -> Result<String, Error> {
    let mut mapped = String::new();
    for c in domain.chars() {
        try!(map_char(c, flags, &mut mapped))
    }
    let normalized: String = mapped.nfc().collect();
    let mut validated = String::new();
    for label in normalized.split('.') {
        if validated.len() > 0 {
            validated.push('.');
        }
        if label.starts_with("xn--") {
            match punycode::decode_to_string(&label["xn--".len()..]) {
                Some(label) => {
                    try!(validate(&label, Uts46Flags {
                        transitional_processing: false,
                        ..flags
                    }));
                    validated.push_str(&label)
                }
                None => return Err(Error::PunycodeError),
            }
        } else {
            try!(validate(label, flags));
            validated.push_str(label)
        }
    }
    Ok(validated)
}

#[derive(Copy, Clone)]
pub struct Uts46Flags {
   pub use_std3_ascii_rules: bool,
   pub transitional_processing: bool,
   pub verify_dns_length: bool,
}

pub enum Error {
    PunycodeError,
    ValidityCriteria,
    DissallowedByStd3AsciiRules,
    DissallowedMappedInStd3,
    DissallowedCharacter,
    TooLongForDns,
}

/// http://www.unicode.org/reports/tr46/#ToASCII
pub fn uts46_to_ascii(domain: &str, flags: Uts46Flags) -> Result<String, Error> {
    let mut result = String::new();
    for label in try!(uts46_processing(domain, flags)).split('.') {
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
                None => return Err(Error::PunycodeError)
            }
        }
    }

    if flags.verify_dns_length {
        let domain = if result.ends_with(".") { &result[..result.len()-1]  } else { &*result };
        if domain.len() < 1 || domain.len() > 253 ||
                domain.split('.').any(|label| label.len() < 1 || label.len() > 63) {
            return Err(Error::TooLongForDns)
        }
    }
    Ok(result)
}

/// https://url.spec.whatwg.org/#concept-domain-to-ascii
pub fn domain_to_ascii(domain: &str) -> Result<String, Error> {
    uts46_to_ascii(domain, Uts46Flags {
        use_std3_ascii_rules: false,
        transitional_processing: true,
        verify_dns_length: false,
    })
}

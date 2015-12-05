//! International domain names
//!
//! https://url.spec.whatwg.org/#idna

use idna_mapping::{TABLE, Mapping};
use punycode;
use std::ascii::AsciiExt;
use unicode_normalization::UnicodeNormalization;

fn map_char(codepoint: char, flags: Uts46Flags, output: &mut String) -> Result<(), Error> {
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

    match TABLE[min].mapping {
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

/// http://www.unicode.org/reports/tr46/#Processing
fn uts46_processing(domain: &str, flags: Uts46Flags) -> Result<String, Error> {
    let mut mapped = String::new();
    for c in domain.chars() {
        try!(map_char(c, flags, &mut mapped))
    }
    Ok(mapped.nfc().collect())
    // FIXME: steps 3 & 4: Break & Convert/Validate
}

#[derive(Copy, Clone)]
pub struct Uts46Flags {
   pub use_std3_ascii_rules: bool,
   pub transitional_processing: bool,
   pub verify_dns_length: bool,
}

pub enum Error {
    PunycodeEncodingError,
    DissallowedByStd3AsciiRules,
    DissallowedMappedInStd3,
    DissallowedCharacter,
    TooLongForDns,
}

/// http://www.unicode.org/reports/tr46/#ToASCII
pub fn uts46_to_ascii(domain: &str, flags: Uts46Flags) -> Result<String, Error> {
    let mut result = String::new();
    for label in try!(uts46_processing(domain, flags)).split('.') {
        if label.is_ascii() {
            if result.len() > 0 {
                result.push('.');
            }
            result.push_str(label);
        } else {
            match punycode::encode_str(label) {
                Some(x) => {
                    if result.len() > 0 {
                        result.push('.');
                    }
                    result.push_str("xn--");
                    result.push_str(&x);
                },
                None => return Err(Error::PunycodeEncodingError)
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

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;

fuzz_target!(|data: &[u8]| {
    // Test domain_to_ascii_cow (primary entry point, takes &[u8])
    let _ = idna::domain_to_ascii_cow(data, idna::AsciiDenyList::URL);
    let _ = idna::domain_to_ascii_cow(data, idna::AsciiDenyList::EMPTY);
    let _ = idna::domain_to_ascii_cow(data, idna::AsciiDenyList::STD3);

    let Ok(utf8) = str::from_utf8(data) else {
        return;
    };

    // Test domain_to_ascii (takes &str)
    let ascii_result = idna::domain_to_ascii(utf8);
    let strict_result = idna::domain_to_ascii_strict(utf8);

    // Roundtrip invariant: if we can convert to ASCII, converting to Unicode
    // and back to ASCII should produce the same result
    if let Ok(ref ascii) = ascii_result {
        let (unicode, unicode_result) = idna::domain_to_unicode(ascii);
        if unicode_result.is_ok() {
            if let Ok(back_to_ascii) = idna::domain_to_ascii(&unicode) {
                assert_eq!(
                    ascii.to_lowercase(),
                    back_to_ascii.to_lowercase(),
                    "IDNA roundtrip mismatch: input={:?}, ascii={:?}, unicode={:?}, back={:?}",
                    utf8,
                    ascii,
                    unicode,
                    back_to_ascii
                );
            }
        }
    }

    // Consistency: strict mode should be a subset of non-strict
    if strict_result.is_ok() {
        assert!(
            ascii_result.is_ok(),
            "strict succeeded but non-strict failed for {:?}",
            utf8
        );
    }

    // Test domain_to_unicode
    let (unicode_str, _result) = idna::domain_to_unicode(utf8);

    // The Unicode result should itself be valid UTF-8 (it's a String)
    let _ = unicode_str.len();

    // Test Punycode encode/decode roundtrip
    if let Some(encoded) = idna::punycode::encode_str(utf8) {
        if let Some(decoded) = idna::punycode::decode_to_string(&encoded) {
            assert_eq!(
                utf8, decoded,
                "Punycode roundtrip mismatch: input={:?}, encoded={:?}, decoded={:?}",
                utf8, encoded, decoded
            );
        }
    }
});

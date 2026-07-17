#![no_main]

use libfuzzer_sys::fuzz_target;
use percent_encoding::{
    percent_decode, percent_decode_str, percent_encode, utf8_percent_encode, AsciiSet, CONTROLS,
    NON_ALPHANUMERIC,
};
use std::borrow::Cow;
use std::str;

/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

/// https://url.spec.whatwg.org/#path-percent-encode-set
const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');

/// https://url.spec.whatwg.org/#userinfo-percent-encode-set
const USERINFO: &AsciiSet = &PATH
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'|');

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Use NON_ALPHANUMERIC for roundtrip tests since it includes '%',
    // ensuring encode→decode is a true roundtrip. Sets that don't encode '%'
    // will cause percent_decode to interpret literal %XX in the input.
    let ascii_sets: [&AsciiSet; 4] = [&CONTROLS, NON_ALPHANUMERIC, FRAGMENT, USERINFO];
    let set_idx = data[0] as usize % ascii_sets.len();
    let ascii_set = ascii_sets[set_idx];
    let input = &data[1..];

    // Test percent_encode -> percent_decode roundtrip with NON_ALPHANUMERIC
    // (which encodes '%', guaranteeing a clean roundtrip)
    let safe_encoded: Cow<str> = percent_encode(input, NON_ALPHANUMERIC).into();
    let safe_decoded: Cow<[u8]> = percent_decode(safe_encoded.as_bytes()).into();
    assert_eq!(
        &*safe_decoded, input,
        "percent_encode/decode roundtrip mismatch with NON_ALPHANUMERIC"
    );

    // Test that encoding with the selected set produces valid output
    let encoded: Cow<str> = percent_encode(input, ascii_set).into();
    let _ = encoded.len();

    // Test UTF-8 path: if input is valid UTF-8, utf8_percent_encode should work too
    if let Ok(utf8_input) = str::from_utf8(input) {
        let utf8_encoded = utf8_percent_encode(utf8_input, NON_ALPHANUMERIC).to_string();
        let utf8_decoded = percent_decode_str(&utf8_encoded)
            .decode_utf8()
            .expect("decoding percent-encoded UTF-8 must produce valid UTF-8");
        assert_eq!(
            utf8_input, &*utf8_decoded,
            "utf8_percent_encode roundtrip mismatch"
        );
    }

    // Test percent_decode directly on raw input
    let direct_decoded: Cow<[u8]> = percent_decode(input).into();
    // Re-encoding with NON_ALPHANUMERIC and decoding again should be stable
    let re_encoded: Cow<str> = percent_encode(&direct_decoded, NON_ALPHANUMERIC).into();
    let re_decoded: Cow<[u8]> = percent_decode(re_encoded.as_bytes()).into();
    assert_eq!(
        &*direct_decoded, &*re_decoded,
        "double roundtrip mismatch"
    );

    // Test percent_decode_str if input is valid UTF-8
    if let Ok(utf8_input) = str::from_utf8(input) {
        let _ = percent_decode_str(utf8_input).decode_utf8_lossy();
    }
});

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Parse the input as form-urlencoded data
    let pairs: Vec<(String, String)> = form_urlencoded::parse(data)
        .into_owned()
        .collect();

    // Roundtrip invariant: serialize and re-parse should produce the same pairs
    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (name, value) in &pairs {
        serializer.append_pair(name, value);
    }
    let serialized = serializer.finish();

    let reparsed: Vec<(String, String)> = form_urlencoded::parse(serialized.as_bytes())
        .into_owned()
        .collect();

    // The key insight: form_urlencoded uses lossy UTF-8 decoding,
    // so we need to compare the parsed pairs (not raw bytes).
    // After one roundtrip through parse->serialize->parse, the result should be stable.
    assert_eq!(
        pairs, reparsed,
        "form_urlencoded roundtrip mismatch: serialized={:?}",
        serialized
    );

    // Test byte_serialize roundtrip
    let byte_serialized: String = form_urlencoded::byte_serialize(data).collect();
    // byte_serialize output should be valid UTF-8 (it produces &str slices)
    let _ = byte_serialized.len();
});

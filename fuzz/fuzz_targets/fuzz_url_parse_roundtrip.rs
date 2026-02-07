#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;
use url::Url;

fuzz_target!(|data: &[u8]| {
    let Ok(utf8) = str::from_utf8(data) else {
        return;
    };

    // Parse the input as a URL
    let Ok(parsed) = Url::parse(utf8) else {
        return;
    };

    // Roundtrip invariant: serializing and re-parsing must produce the same URL
    let serialized = parsed.as_str();
    let reparsed = Url::parse(serialized).expect("re-parsing a serialized URL must succeed");
    assert_eq!(
        parsed.as_str(),
        reparsed.as_str(),
        "roundtrip mismatch for input: {:?}",
        utf8
    );

    // Component invariant: individual components must be consistent
    assert_eq!(parsed.scheme(), reparsed.scheme());
    assert_eq!(parsed.username(), reparsed.username());
    assert_eq!(parsed.password(), reparsed.password());
    assert_eq!(parsed.host_str(), reparsed.host_str());
    assert_eq!(parsed.port(), reparsed.port());
    assert_eq!(parsed.path(), reparsed.path());
    assert_eq!(parsed.query(), reparsed.query());
    assert_eq!(parsed.fragment(), reparsed.fragment());

    // Join invariant: joining an absolute URL with itself yields the same URL
    if let Ok(joined) = parsed.join(serialized) {
        assert_eq!(joined.as_str(), serialized);
    }

    // Origin consistency
    let _ = parsed.origin();
});

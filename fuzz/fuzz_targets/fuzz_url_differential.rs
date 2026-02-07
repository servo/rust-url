#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;
use url::Url;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let Ok(utf8) = str::from_utf8(data) else {
        return;
    };

    // Split input into a base URL part and a relative part
    let split = (data[0] as usize) % utf8.len().max(1);
    let (base_str, relative_str) = utf8.split_at(split);

    // Try parsing base as absolute URL
    let Ok(base) = Url::parse(base_str) else {
        return;
    };

    // Test relative URL resolution
    if let Ok(resolved) = base.join(relative_str) {
        // The resolved URL must be valid
        let serialized = resolved.as_str();
        let reparsed =
            Url::parse(serialized).expect("re-parsing a resolved URL must succeed");
        assert_eq!(resolved.as_str(), reparsed.as_str());

        // make_relative + join should roundtrip for non-opaque paths
        if !base.cannot_be_a_base() && !resolved.cannot_be_a_base() {
            if let Some(relative) = resolved.make_relative(&base) {
                // Re-resolving the relative URL from base should give the same result
                if let Ok(re_resolved) = base.join(&relative) {
                    // Scheme and host should match
                    assert_eq!(re_resolved.scheme(), resolved.scheme());
                    assert_eq!(re_resolved.host_str(), resolved.host_str());
                }
            }
        }
    }

    // Test parse_with_params
    if utf8.len() < 500 {
        let params = [("key", "value"), ("a", "b")];
        if let Ok(with_params) = Url::parse_with_params(utf8, &params) {
            let query = with_params.query().unwrap_or("");
            assert!(query.contains("key=value"));
            assert!(query.contains("a=b"));
        }
    }
});

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: String| {
    if let Ok(parsed) = url::Url::parse(data.as_str()) {
        let as_str = parsed.as_str();
        let parsed_again = url::Url::parse(as_str).unwrap();
        assert_eq!(parsed, parsed_again);
    }
});

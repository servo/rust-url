#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;
use url::Url;

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }

    let Ok(utf8) = str::from_utf8(&data[2..]) else {
        return;
    };

    // Use first byte to select a base URL, second byte to select which setter to test
    let base_urls = [
        "https://example.com/path?query#fragment",
        "http://user:pass@host:8080/a/b/c",
        "ftp://files.example.com/pub",
        "file:///tmp/test",
        "custom://example",
    ];

    let base_idx = data[0] as usize % base_urls.len();
    let setter_idx = data[1] % 10;

    let mut url = Url::parse(base_urls[base_idx]).unwrap();
    let original = url.as_str().to_string();

    match setter_idx {
        0 => {
            let _ = url.set_scheme(utf8);
        }
        1 => {
            let _ = url.set_host(Some(utf8));
        }
        2 => {
            let _ = url.set_host(None);
        }
        3 => {
            let _ = url.set_username(utf8);
        }
        4 => {
            let _ = url.set_password(Some(utf8));
        }
        5 => {
            url.set_path(utf8);
        }
        6 => {
            url.set_query(Some(utf8));
        }
        7 => {
            url.set_fragment(Some(utf8));
        }
        8 => {
            if let Ok(port) = utf8.parse::<u16>() {
                let _ = url.set_port(Some(port));
            }
        }
        9 => {
            if let Ok(mut segs) = url.path_segments_mut() {
                segs.push(utf8);
            }
        }
        _ => {}
    }

    // After mutation, the URL must still be valid (roundtrip)
    let modified = url.as_str().to_string();
    let reparsed = Url::parse(&modified).unwrap_or_else(|e| {
        panic!(
            "URL became invalid after mutation: {:?}\noriginal: {}\nmodified: {}\nerror: {}",
            setter_idx, original, modified, e
        );
    });
    assert_eq!(url.as_str(), reparsed.as_str());
});

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str;

fuzz_target!(|data: &[u8]| {
    let Ok(utf8) = str::from_utf8(data) else {
        return;
    };

    let Ok(data_url) = data_url::DataUrl::process(utf8) else {
        return;
    };

    // Access MIME type (should not panic)
    let mime = data_url.mime_type();
    let _ = mime.type_.len();
    let _ = mime.subtype.len();
    for (name, value) in &mime.parameters {
        let _ = name.len();
        let _ = value.len();
    }

    // Decode body (should not panic)
    match data_url.decode_to_vec() {
        Ok((body, fragment)) => {
            // Body must be valid bytes
            let _ = body.len();
            if let Some(frag) = fragment {
                // Fragment percent-encoding should produce valid UTF-8
                let _ = frag.to_percent_encoded();
            }
        }
        Err(_) => {
            // Base64 decode errors are expected for malformed input
        }
    }

    // Test streaming decode
    let mut chunks = Vec::new();
    let _ = data_url.decode(|bytes| {
        chunks.push(bytes.to_vec());
        Ok::<(), std::convert::Infallible>(())
    });

    // Test forgiving_base64 directly
    let _ = data_url::forgiving_base64::decode_to_vec(data);
});

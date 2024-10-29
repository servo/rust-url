#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate url;

use std::str;
use url::{Url, quirks, Origin, Host, Position};

fuzz_target!(|data: &[u8]| {
    // Initialisation
    let utf8 = match str::from_utf8(data) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut url_parse_attempt = Url::parse(utf8);

    // Randomly fuzz functions
    match data.get(0) {
        Some(&choice) => match choice % 20 {
            0 => {
                if let Ok(parsed_url) = &url_parse_attempt {
                    let _ = parsed_url.query();
                }
            },
            1 => {
                if let Ok(parsed_url) = &url_parse_attempt {
                    let _ = parsed_url.fragment();
                }
            },
            2 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = quirks::set_protocol(parsed_url, utf8);
                }
            },
            3 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = quirks::set_username(parsed_url, utf8);
                }
            },
            4 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = quirks::set_password(parsed_url, utf8);
                }
            },
            5 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    quirks::set_search(parsed_url, utf8);
                }
            },
            6 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    quirks::set_hash(parsed_url, utf8);
                }
            },
            7 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    parsed_url.set_scheme("https").ok();
                }
            },
            8 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.set_host(Some("example.com"));
                }
            },
            9 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.set_port(Some(8080));
                }
            },
            10 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.set_path("/test/path");
                }
            },
            11 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.path_segments_mut().map(|mut segments| {
                        segments.push("segment1");
                        segments.push("segment2");
                    });
                }
            },
            12 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.set_query(Some("key=value"));
                }
            },
            13 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.set_fragment(Some("fragment"));
                }
            },
            14 => {
                if let Ok(parsed_url) = &url_parse_attempt {
                    if let Some(domain) = parsed_url.host_str() {
                        let _ = Host::parse(domain);
                    }
                }
            },
            15 => {
                if let Ok(parsed_url) = &url_parse_attempt {
                    let _ = parsed_url.origin().ascii_serialization();
                }
            },
            16 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.join("/relative/path");
                }
            },
            17 => {
                if let Ok(parsed_url) = &mut url_parse_attempt {
                    let _ = parsed_url.make_relative(&Url::parse("https://example.com/base").unwrap());
                }
            },
            18 => {
                if let Ok(parsed_url) = &url_parse_attempt {
                    let _ = &parsed_url[Position::BeforeHost..Position::AfterPort];
                }
            },
            19 => {
                if let Ok(parsed_url) = &url_parse_attempt {
                    let _ = &parsed_url[Position::BeforeScheme..];
                }
            },
            _ => {},
        },
        None => {},
    }
});

extern crate url;
extern crate rustc_serialize;

use url::punycode::{decode, encode_str};
use rustc_serialize::json::{Json, Object};

fn one_test(description: &str, decoded: &str, encoded: &str) {
    match decode(encoded) {
        None => panic!("Decoding {} failed.", encoded),
        Some(result) => {
            let result = result.into_iter().collect::<String>();
            assert!(result == decoded,
                    format!("Incorrect decoding of {}:\n   {}\n!= {}\n{}",
                            encoded, result, decoded, description))
        }
    }

    match encode_str(decoded) {
        None => panic!("Encoding {} failed.", decoded),
        Some(result) => {
            assert!(result == encoded,
                    format!("Incorrect encoding of {}:\n   {}\n!= {}\n{}",
                            decoded, result, encoded, description))
        }
    }
}

fn get_string<'a>(map: &'a Object, key: &str) -> &'a str {
    match map.get(&key.to_string()) {
        Some(&Json::String(ref s)) => s,
        None => "",
        _ => panic!(),
    }
}

#[test]
fn test_punycode() {

    match Json::from_str(include_str!("punycode_tests.json")) {
        Ok(Json::Array(tests)) => for test in &tests {
            match test {
                &Json::Object(ref o) => one_test(
                    get_string(o, "description"),
                    get_string(o, "decoded"),
                    get_string(o, "encoded")
                ),
                _ => panic!(),
            }
        },
        other => panic!("{:?}", other)
    }
}

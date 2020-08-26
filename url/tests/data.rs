// Copyright 2013-2014 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Data-driven tests

use serde_json::Value;
use std::str::FromStr;
use url::{quirks, Url};

fn check_invariants(url: &Url, name: &str, comment: Option<&str>) -> bool {
    let mut passed = true;
    if let Err(e) = url.check_invariants() {
        passed = false;
        eprint_failure(
            format!("  failed: invariants checked -> {:?}", e),
            name,
            comment,
        );
    }

    #[cfg(feature = "serde")]
    {
        let bytes = serde_json::to_vec(url).unwrap();
        let new_url: Url = serde_json::from_slice(&bytes).unwrap();
        passed &= test_eq(url, &new_url, name, comment);
    }

    passed
}

struct ExpectedAttributes {
    href: String,
    origin: Option<String>,
    protocol: String,
    username: String,
    password: String,
    host: String,
    hostname: String,
    port: String,
    pathname: String,
    search: String,
    hash: String,
}

trait JsonExt {
    fn take_key(&mut self, key: &str) -> Option<Value>;
    fn string(self) -> String;
    fn take_string(&mut self, key: &str) -> String;
}

impl JsonExt for Value {
    fn take_key(&mut self, key: &str) -> Option<Value> {
        self.as_object_mut().unwrap().remove(key)
    }

    fn string(self) -> String {
        if let Value::String(s) = self {
            s
        } else {
            panic!("Not a Value::String")
        }
    }

    fn take_string(&mut self, key: &str) -> String {
        self.take_key(key).unwrap().string()
    }
}

#[test]
fn urltestdata() {
    // Copied form https://github.com/w3c/web-platform-tests/blob/master/url/
    let mut json = Value::from_str(include_str!("urltestdata.json"))
        .expect("JSON parse error in urltestdata.json");
    let mut passed = true;
    for entry in json.as_array_mut().unwrap() {
        if entry.is_string() {
            continue; // ignore comments
        }
        let base = entry.take_string("base");
        let input = entry.take_string("input");
        let expected = if entry.take_key("failure").is_some() {
            Err(())
        } else {
            Ok(ExpectedAttributes {
                href: entry.take_string("href"),
                origin: entry.take_key("origin").map(|s| s.string()),
                protocol: entry.take_string("protocol"),
                username: entry.take_string("username"),
                password: entry.take_string("password"),
                host: entry.take_string("host"),
                hostname: entry.take_string("hostname"),
                port: entry.take_string("port"),
                pathname: entry.take_string("pathname"),
                search: entry.take_string("search"),
                hash: entry.take_string("hash"),
            })
        };

        let base = match Url::parse(&base) {
            Ok(base) => base,
            Err(_) if expected.is_err() => continue,
            Err(message) => {
                eprint_failure(
                    format!("  failed: error parsing base {:?}: {}", base, message),
                    &format!("parse base for {:?}", input),
                    None,
                );
                passed = false;
                continue;
            }
        };

        let (url, expected) = match (base.join(&input), expected) {
            (Ok(url), Ok(expected)) => (url, expected),
            (Err(_), Err(())) => continue,
            (Err(message), Ok(_)) => {
                eprint_failure(
                    format!("  failed: {}", message),
                    &format!("parse URL for {:?}", input),
                    None,
                );
                passed = false;
                continue;
            }
            (Ok(_), Err(())) => {
                eprint_failure(
                    format!("  failed: expected parse error for URL {:?}", input),
                    &format!("parse URL for {:?}", input),
                    None,
                );
                passed = false;
                continue;
            }
        };

        passed &= check_invariants(&url, &format!("invariants for {:?}", input), None);

        macro_rules! assert_attributes {
            ($($attr: ident)+) => {$(test_eq_eprint(
                expected.$attr,
                quirks::$attr(&url),
                &format!("{:?} - {}", input, stringify!($attr)),
                None,
            ))&+}
        }

        passed &= assert_attributes!(
            href protocol username password host hostname port pathname search hash
        );

        if let Some(expected_origin) = expected.origin {
            passed &= test_eq_eprint(
                expected_origin,
                &quirks::origin(&url),
                &format!("origin for {:?}", input),
                None,
            );
        }
    }

    assert!(passed)
}

#[test]
fn setters_tests() {
    let mut json = Value::from_str(include_str!("setters_tests.json"))
        .expect("JSON parse error in setters_tests.json");

    macro_rules! setter {
        ($attr: expr, $setter: ident) => {{
            let mut tests = json.take_key($attr).unwrap();
            let mut passed = true;
            for mut test in tests.as_array_mut().unwrap().drain(..) {
                let comment = test.take_key("comment").map(|s| s.string());
                let href = test.take_string("href");
                let new_value = test.take_string("new_value");
                let name = format!("{:?}.{} = {:?}", href, $attr, new_value);
                let mut expected = test.take_key("expected").unwrap();

                let mut url = Url::parse(&href).unwrap();
                let comment_ref = comment.as_deref();
                passed &= check_invariants(&url, &name, comment_ref);
                let _ = quirks::$setter(&mut url, &new_value);

                passed &= assert_attributes!(&name, comment_ref, url, expected,
                    href protocol username password host hostname port pathname search hash);
                passed &= check_invariants(&url, &name, comment_ref);
            }
            passed
        }}
    }

    macro_rules! assert_attributes {
        ($name: expr, $comment: expr, $url: expr, $expected: expr, $($attr: ident)+) => {
            $(match $expected.take_key(stringify!($attr)) {
                Some(value) => test_eq_eprint(
                    value.string(),
                    quirks::$attr(&$url),
                    $name,
                    $comment,
                ),
                None => true,
            })&+
        }
    }

    let mut passed = true;
    passed &= setter!("protocol", set_protocol);
    passed &= setter!("username", set_username);
    passed &= setter!("password", set_password);
    passed &= setter!("hostname", set_hostname);
    passed &= setter!("host", set_host);
    passed &= setter!("port", set_port);
    passed &= setter!("pathname", set_pathname);
    passed &= setter!("search", set_search);
    passed &= setter!("hash", set_hash);
    assert!(passed);
}

fn test_eq_eprint(expected: String, actual: &str, name: &str, comment: Option<&str>) -> bool {
    if expected == actual {
        return true;
    }
    eprint_failure(
        format!("expected: {}\n  actual: {}", expected, actual),
        name,
        comment,
    );
    false
}

fn eprint_failure(err: String, name: &str, comment: Option<&str>) {
    eprintln!("    test: {}\n{}", name, err);
    if let Some(comment) = comment {
        eprintln!("{}\n", comment);
    } else {
        eprintln!("");
    }
}

// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tests copied form https://github.com/w3c/web-platform-tests/blob/master/url/

extern crate rustc_serialize;
extern crate test;
extern crate url;

use rustc_serialize::json::Json;
use url::{Url, WebIdl};


fn run_one(input: String, base: String, expected: Result<TestCase, ()>) {
    let base = match Url::parse(&base) {
        Ok(base) => base,
        Err(message) => panic!("Error parsing base {:?}: {}", base, message)
    };
    let (url, expected) = match (base.join(&input), expected) {
        (Ok(url), Ok(expected)) => (url, expected),
        (Err(_), Err(())) => return,
        (Err(message), Ok(_)) => panic!("Error parsing URL {:?}: {}", input, message),
        (Ok(_), Err(())) => panic!("Expected a parse error for URL {:?}", input),
    };

    macro_rules! assert_getter {
        ($attribute: ident) => { assert_getter!($attribute, expected.$attribute) };
        ($attribute: ident, $expected: expr) => {
            {
                let a = WebIdl::$attribute(&url);
                let b = $expected;
                assert!(a == b, "{:?} != {:?} for URL {:?}", a, b, url);
            }
        }
    }

    assert_getter!(href);
    if let Some(expected_origin) = expected.origin {
        assert_getter!(origin, expected_origin);
    }
    assert_getter!(protocol);
    assert_getter!(username);
    assert_getter!(password);
    assert_getter!(host);
    assert_getter!(hostname);
    assert_getter!(port);
    assert_getter!(pathname);
    assert_getter!(search);
    assert_getter!(hash);
}

struct TestCase {
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

fn main() {
    let json = Json::from_str(include_str!("urltestdata.json"))
        .expect("JSON parse error in urltestdata.json");
    let tests = json.as_array().unwrap().iter().filter_map(|entry| {
        if entry.is_string() {
            return None  // ignore comments
        }
        let string = |key| entry.find(key).unwrap().as_string().unwrap().to_owned();
        let base = string("base");
        let input = string("input");
        let expected = if entry.find("failure").is_some() {
            Err(())
        } else {
            Ok(TestCase {
                href: string("href"),
                origin: entry.find("origin").map(|j| j.as_string().unwrap().to_owned()),
                protocol: string("protocol"),
                username: string("username"),
                password: string("password"),
                host: string("host"),
                hostname: string("hostname"),
                port: string("port"),
                pathname: string("pathname"),
                search: string("search"),
                hash: string("hash"),
            })
        };
        Some(test::TestDescAndFn {
            desc: test::TestDesc {
                name: test::DynTestName(format!("{:?} @ base {:?}", input, base)),
                ignore: false,
                should_panic: test::ShouldPanic::No,
            },
            testfn: test::TestFn::dyn_test_fn(move || run_one(input, base, expected)),
        })
    }).collect();
    test::test_main(&std::env::args().collect::<Vec<_>>(), tests)
}

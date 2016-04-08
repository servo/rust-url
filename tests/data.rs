// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Data-driven tests

extern crate rustc_serialize;
extern crate test;
extern crate url;

use rustc_serialize::json::Json;
use url::{Url, Position};


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

    url.assert_invariants();

    macro_rules! assert_eq {
        ($expected: expr, $got: expr) => {
            {
                let expected = $expected;
                let got = $got;
                assert!(expected == got, "{:?} != {} {:?} for URL {:?}",
                        got, stringify!($expected), expected, url);
            }
        }
    }

    assert_eq!(expected.href, url.as_str());
    if let Some(expected_origin) = expected.origin {
        assert_eq!(expected_origin, url.origin().unicode_serialization());
    }
    assert_eq!(expected.protocol, &url.as_str()[..url.scheme().len() + ":".len()]);
    assert_eq!(expected.username, url.username());
    assert_eq!(expected.password, url.password().unwrap_or(""));
    assert_eq!(expected.host, &url[Position::BeforeHost..Position::AfterPort]);
    assert_eq!(expected.hostname, url.host_str().unwrap_or(""));
    assert_eq!(expected.port, &url[Position::BeforePort..Position::AfterPort]);
    assert_eq!(expected.pathname, url.path());
    assert_eq!(expected.search, trim(&url[Position::AfterPath..Position::AfterQuery]));
    assert_eq!(expected.hash, trim(&url[Position::AfterQuery..]));
}

fn trim(s: &str) -> &str {
    if s.len() == 1 {
        ""
    } else {
        s
    }
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
    // Copied form https://github.com/w3c/web-platform-tests/blob/master/url/
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

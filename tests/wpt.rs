// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tests copied form https://github.com/w3c/web-platform-tests/blob/master/url/

extern crate test;
extern crate url;

use std::char;
use url::{RelativeSchemeData, SchemeData, Url};


fn run_one(entry: Entry) {
    // FIXME: Don’t re-indent to make merging the 1.0 branch easier.
    {
        let Entry {
            input,
            base,
            scheme: expected_scheme,
            username: expected_username,
            password: expected_password,
            host: expected_host,
            port: expected_port,
            path: expected_path,
            query: expected_query,
            fragment: expected_fragment,
            expected_failure,
        } = entry;
        let base = match Url::parse(&base) {
            Ok(base) => base,
            Err(message) => panic!("Error parsing base {}: {}", base, message)
        };
        let url = base.join(&input);
        if expected_scheme.is_none() {
            if url.is_ok() && !expected_failure {
                panic!("Expected a parse error for URL {}", input);
            }
            return
        }
        let Url { scheme, scheme_data, query, fragment, .. } = match url {
            Ok(url) => url,
            Err(message) => {
                if expected_failure {
                    return
                } else {
                    panic!("Error parsing URL {}: {}", input, message)
                }
            }
        };

        macro_rules! assert_eq {
            ($a: expr, $b: expr) => {
                {
                    let a = $a;
                    let b = $b;
                    if a != b {
                        if expected_failure {
                            return
                        } else {
                            panic!("{:?} != {:?}", a, b)
                        }
                    }
                }
            }
        }

        assert_eq!(Some(scheme), expected_scheme);
        match scheme_data {
            SchemeData::Relative(RelativeSchemeData {
                username, password, host, port, default_port: _, path,
            }) => {
                assert_eq!(username, expected_username);
                assert_eq!(password, expected_password);
                let host = host.serialize();
                assert_eq!(host, expected_host);
                assert_eq!(port, expected_port);
                assert_eq!(Some(format!("/{}", str_join(&path, "/"))), expected_path);
            },
            SchemeData::NonRelative(scheme_data) => {
                assert_eq!(Some(scheme_data), expected_path);
                assert_eq!(String::new(), expected_username);
                assert_eq!(None, expected_password);
                assert_eq!(String::new(), expected_host);
                assert_eq!(None, expected_port);
            },
        }
        fn opt_prepend(prefix: &str, opt_s: Option<String>) -> Option<String> {
            opt_s.map(|s| format!("{}{}", prefix, s))
        }
        assert_eq!(opt_prepend("?", query), expected_query);
        assert_eq!(opt_prepend("#", fragment), expected_fragment);

        assert!(!expected_failure, "Unexpected success for {}", input);
    }
}

// FIMXE: Remove this when &[&str]::join (the new name) lands in the stable channel.
#[allow(deprecated)]
fn str_join<T: ::std::borrow::Borrow<str>>(pieces: &[T], separator: &str) -> String {
    pieces.connect(separator)
}

struct Entry {
    input: String,
    base: String,
    scheme: Option<String>,
    username: String,
    password: Option<String>,
    host: String,
    port: Option<u16>,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
    expected_failure: bool,
}

fn parse_test_data(input: &str) -> Vec<Entry> {
    let mut tests: Vec<Entry> = Vec::new();
    for line in input.lines() {
        if line == "" || line.starts_with("#") {
            continue
        }
        let mut pieces = line.split(' ').collect::<Vec<&str>>();
        let expected_failure = pieces[0] == "XFAIL";
        if expected_failure {
            pieces.remove(0);
        }
        let input = unescape(pieces.remove(0));
        let mut test = Entry {
            input: input,
            base: if pieces.is_empty() || pieces[0] == "" {
                tests.last().unwrap().base.clone()
            } else {
                unescape(pieces.remove(0))
            },
            scheme: None,
            username: String::new(),
            password: None,
            host: String::new(),
            port: None,
            path: None,
            query: None,
            fragment: None,
            expected_failure: expected_failure,
        };
        for piece in pieces {
            if piece == "" || piece.starts_with("#") {
                continue
            }
            let colon = piece.find(':').unwrap();
            let value = unescape(&piece[colon + 1..]);
            match &piece[..colon] {
                "s" => test.scheme = Some(value),
                "u" => test.username = value,
                "pass" => test.password = Some(value),
                "h" => test.host = value,
                "port" => test.port = Some(value.parse().unwrap()),
                "p" => test.path = Some(value),
                "q" => test.query = Some(value),
                "f" => test.fragment = Some(value),
                _ => panic!("Invalid token")
            }
        }
        tests.push(test)
    }
    tests
}

fn unescape(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars();
    loop {
        match chars.next() {
            None => return output,
            Some(c) => output.push(
                if c == '\\' {
                    match chars.next().unwrap() {
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        's' => ' ',
                        't' => '\t',
                        'f' => '\x0C',
                        'u' => {
                            char::from_u32((((
                                chars.next().unwrap().to_digit(16).unwrap()) * 16 +
                                chars.next().unwrap().to_digit(16).unwrap()) * 16 +
                                chars.next().unwrap().to_digit(16).unwrap()) * 16 +
                                chars.next().unwrap().to_digit(16).unwrap()).unwrap()
                        }
                        _ => panic!("Invalid test data input"),
                    }
                } else {
                    c
                }
            )
        }
    }
}

fn make_test(entry: Entry) -> test::TestDescAndFn {
    test::TestDescAndFn {
        desc: test::TestDesc {
            name: test::DynTestName(format!("{:?} base {:?}", entry.input, entry.base)),
            ignore: false,
            should_panic: test::ShouldPanic::No,
        },
        testfn: test::TestFn::dyn_test_fn(move || run_one(entry)),
    }

}

fn main() {
    test::test_main(
        &std::env::args().collect::<Vec<_>>(),
        parse_test_data(include_str!("urltestdata.txt")).into_iter().map(make_test).collect(),
    )
}

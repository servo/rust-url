// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::char;
use std::u32;
use super::{Url, RelativeSchemeData, SchemeRelativeUrl, OtherSchemeData};


#[test]
fn test_url_parsing() {
    for test in parse_test_data(include_str!("urltestdata.txt")).move_iter() {
        let Test {
            input: input,
            base: base,
            scheme: expected_scheme,
            username: expected_username,
            password: expected_password,
            host: expected_host,
            port: expected_port,
            path: expected_path,
            query: expected_query,
            fragment: expected_fragment,
            expected_failure: expected_failure,
        } = test;
        let base = match Url::parse(base.as_slice(), None) {
            Ok(base) => base,
            Err(message) => fail!("Error parsing base {}: {}", base, message)
        };
        let url = Url::parse(input.as_slice(), Some(&base));
        if expected_scheme.is_none() {
            if url.is_ok() && !expected_failure {
                fail!("Expected a parse error for URL {}", input);
            }
            continue
        }
        let Url { scheme, scheme_data, query, fragment, .. } = match url {
            Ok(url) => url,
            Err(message) => {
                if expected_failure {
                    continue
                } else {
                    fail!("Error parsing URL {}: {}", input, message)
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
                            continue
                        } else {
                            fail!("{} != {}", a, b)
                        }
                    }
                }
            }
        }

        assert_eq!(Some(scheme), expected_scheme);
        match scheme_data {
            RelativeSchemeData(SchemeRelativeUrl { username, password, host, port, path }) => {
                assert_eq!(username, expected_username);
                assert_eq!(password, expected_password);
                let host = host.serialize();
                assert_eq!(host, expected_host)
                assert_eq!(port, expected_port);
                assert_eq!(Some("/".to_string().append(path.connect("/").as_slice())),
                           expected_path);
            },
            OtherSchemeData(scheme_data) => {
                assert_eq!(Some(scheme_data), expected_path);
                assert_eq!(String::new(), expected_username);
                assert_eq!(None, expected_password);
                assert_eq!(String::new(), expected_host);
                assert_eq!(String::new(), expected_port);
            },
        }
        fn opt_prepend(prefix: &str, opt_s: Option<String>) -> Option<String> {
            opt_s.map(|s| prefix.to_string().append(s.as_slice()))
        }
        assert_eq!(opt_prepend("?", query), expected_query);
        assert_eq!(opt_prepend("#", fragment), expected_fragment);

        assert!(!expected_failure, "Unexpected success for {}", input);
    }
}

struct Test {
    input: String,
    base: String,
    scheme: Option<String>,
    username: String,
    password: Option<String>,
    host: String,
    port: String,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
    expected_failure: bool,
}

fn parse_test_data(input: &str) -> Vec<Test> {
    let mut tests: Vec<Test> = Vec::new();
    for line in input.lines() {
        if line == "" || line.starts_with("#") {
            continue
        }
        let mut pieces = line.split(' ').collect::<Vec<&str>>();
        let expected_failure = *pieces.get(0) == "XFAIL";
        if expected_failure {
            pieces.shift();
        }
        let input = unescape(pieces.shift().unwrap());
        let mut test = Test {
            input: input,
            base: if pieces.is_empty() || *pieces.get(0) == "" {
                tests.last().unwrap().base.clone()
            } else {
                unescape(pieces.shift().unwrap())
            },
            scheme: None,
            username: String::new(),
            password: None,
            host: String::new(),
            port: String::new(),
            path: None,
            query: None,
            fragment: None,
            expected_failure: expected_failure,
        };
        for piece in pieces.move_iter() {
            if piece == "" || piece.starts_with("#") {
                continue
            }
            let colon = piece.find(':').unwrap();
            let value = unescape(piece.slice_from(colon + 1));
            match piece.slice_to(colon) {
                "s" => test.scheme = Some(value),
                "u" => test.username = value,
                "pass" => test.password = Some(value),
                "h" => test.host = value,
                "port" => test.port = value,
                "p" => test.path = Some(value),
                "q" => test.query = Some(value),
                "f" => test.fragment = Some(value),
                _ => fail!("Invalid token")
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
            Some(c) => output.push_char(
                if c == '\\' {
                    match chars.next().unwrap() {
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        's' => ' ',
                        't' => '\t',
                        'f' => '\x0C',
                        'u' => {
                            let mut hex = String::new();
                            hex.push_char(chars.next().unwrap());
                            hex.push_char(chars.next().unwrap());
                            hex.push_char(chars.next().unwrap());
                            hex.push_char(chars.next().unwrap());
                            u32::parse_bytes(hex.as_bytes(), 16)
                                .and_then(char::from_u32).unwrap()
                        }
                        _ => fail!("Invalid test data input"),
                    }
                } else {
                    c
                }
            )
        }
    }
}

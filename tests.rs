// Copyright 2013 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::char;
use std::u32;
use super::{URL, RelativeSchemeData, SchemeRelativeURL, UserInfo, OtherSchemeData};


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
            fragment: expected_fragment
        } = test;
        let base = match URL::parse(base, None) {
            Ok(base) => base,
            Err(message) => fail!("Error parsing base {:?}: {}", base, message)
        };
        let url = URL::parse(input, Some(&base));
        if expected_scheme.is_none() {
            assert!(url.is_err(), "Expected a parse error for URL {:?}", input);
            continue
        }
        let URL { scheme, scheme_data, query, fragment } = match url {
            Ok(url) => url,
            Err(message) => fail!("Error parsing URL {:?}: {}", input, message)
        };

        assert_eq!(Some(scheme), expected_scheme);
        match scheme_data {
            RelativeSchemeData(SchemeRelativeURL { userinfo, host, port, path }) => {
                let (username, password) = match userinfo {
                    None => (~"", None),
                    Some(UserInfo { username, password }) => (username, password),
                };
                assert_eq!(username, expected_username);
                assert_eq!(password, expected_password);
                let host = host.serialize();
                assert_eq!(host, expected_host)
                assert_eq!(port, expected_port);
                assert_eq!(Some("/" + path.connect("/")),
                           expected_path);
            },
            OtherSchemeData(scheme_data) => {
                assert_eq!(Some(scheme_data), expected_path);
                assert_eq!(~"", expected_username);
                assert_eq!(None, expected_password);
                assert_eq!(~"", expected_host);
                assert_eq!(~"", expected_port);
            },
        }
        assert_eq!(query.map(|p| "?" + p), expected_query);
        assert_eq!(fragment.map(|p| "#" + p), expected_fragment);
    }
}

struct Test {
    input: ~str,
    base: ~str,
    scheme: Option<~str>,
    username: ~str,
    password: Option<~str>,
    host: ~str,
    port: ~str,
    path: Option<~str>,
    query: Option<~str>,
    fragment: Option<~str>,
}

fn parse_test_data(input: &str) -> ~[Test] {
    let mut tests: ~[Test] = ~[];
    for line in input.lines() {
        if line == "" || line[0] == ('#' as u8) {
            continue
        }
        let mut pieces = line.split(' ').to_owned_vec();
        let input = unescape(pieces.shift());
        let mut test = Test {
            input: input,
            base: if pieces.is_empty() || pieces[0] == "" {
                tests[tests.len() - 1].base.to_owned()
            } else {
                unescape(pieces.shift())
            },
            scheme: None,
            username: ~"",
            password: None,
            host: ~"",
            port: ~"",
            path: None,
            query: None,
            fragment: None,
        };
        for piece in pieces.move_iter() {
            if piece == "" || piece[0] == ('#' as u8) {
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

fn unescape(input: &str) -> ~str {
    let mut output = ~"";
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
                            let mut hex = ~"";
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

// Copyright 2013 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[link(name = "url", vers = "0.1")];
#[crate_type = "lib"];
#[feature(globs)];


pub struct ParsedURL {
    scheme_and_data: Either<(NonRelativeScheme, ~str),
                            (RelativeScheme, SchemeRelativeURL)>,
    query: Option<~str>,
    fragment: Option<~str>,
}

pub enum NonRelativeScheme {
    Data,
    Javascript,
    Mailto,
    OtherScheme(~str),
}

pub enum RelativeScheme {
    FTP,
    File,
    Gopher,
    HTTP,
    HTTPS,
    WS,
    WSS,
}

pub struct SchemeRelativeURL {
    userinfo: Option<UserInfo>,
    host: Host,
    port: Option<~str>,
    path: ~[~str],
}

pub struct UserInfo {
    username: ~str,
    password: Option<~str>,
}

pub enum Host {
    Domain(~[~str]),
    IPv6Address([u16, ..8])
}


pub fn parse_url(input: &str, base_url: Option<ParsedURL>)
          -> Option<ParsedURL> {
    let _ = input;
    let _ = base_url;
    None
}


#[cfg(test)]
mod tests {
    use std::{char, u32};
    use super::*;

    #[test]
    fn test() {
        for Test {
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
        } in parse_test_data(include_str!("urltestdata.txt")).move_iter() {
            let base = parse_url(base, None).unwrap();
            let url = parse_url(input, Some(base)).unwrap();
            assert_eq!(url.query, expected_query);
            assert_eq!(url.fragment, expected_fragment);
            match url.scheme_and_data {
                Left((scheme, scheme_data)) => {
                    let scheme = match scheme {
                        Data => ~"data",
                        Javascript => ~"javascript",
                        Mailto => ~"mailto",
                        OtherScheme(scheme) => scheme,
                    };
                    assert_eq!(Some(scheme), expected_scheme);
                    assert_eq!(Some(scheme_data), expected_path);
                    assert_eq!(expected_username, None);
                    assert_eq!(expected_password, None);
                    assert_eq!(expected_host, None);
                    assert_eq!(expected_port, None);
                },
                Right((scheme, scheme_data)) => {
                    let scheme = match scheme {
                        FTP => ~"ftp",
                        File => ~"file",
                        Gopher => ~"gopher",
                        HTTP => ~"http",
                        HTTPS => ~"https",
                        WS => ~"ws",
                        WSS => ~"wss",
                    };
                    assert_eq!(Some(scheme), expected_scheme);
                    let SchemeRelativeURL {
                        userinfo: userinfo, host: host, port: port, path: path
                    } = scheme_data;
                    match userinfo {
                        Some(UserInfo { username: username, password: password }) => {
                            assert_eq!(Some(username), expected_username);
                            assert_eq!(password, expected_password);
                        },
                        None => {
                            assert_eq!(expected_username, None);
                            assert_eq!(expected_password, None);
                        }
                    }
                    match host {
                        Domain(labels) => assert_eq!(Some(labels.connect(".")), expected_host),
                        IPv6Address(_fields) => fail!("TODO: IPv6 serialization"),
                    }
                    assert_eq!(port, expected_port);
                    assert_eq!(Some(path.connect("/")), expected_path);
                },
            }
        }
    }

    struct Test {
        input: ~str,
        base: ~str,
        scheme: Option<~str>,
        username: Option<~str>,
        password: Option<~str>,
        host: Option<~str>,
        port: Option<~str>,
        path: Option<~str>,
        query: Option<~str>,
        fragment: Option<~str>,
    }

    fn parse_test_data(input: &str) -> ~[Test] {
        let mut tests: ~[Test] = ~[];
        for line in input.line_iter() {
            if line == "" || line[0] == ('#' as u8) {
                continue
            }
            let mut pieces = line.split_iter(' ').to_owned_vec();
            let input = unescape(pieces.shift());
            let mut test = Test {
                input: input,
                base: if pieces.is_empty() {
                    tests[tests.len() - 1].base.to_owned()
                } else {
                    unescape(pieces.shift())
                },
                scheme: None,
                username: None,
                password: None,
                host: None,
                port: None,
                path: None,
                query: None,
                fragment: None,
            };
            for piece in pieces.move_iter() {
                if piece != "" || piece[0] == ('#' as u8) {
                    continue
                }
                let colon = piece.find(':').unwrap();
                let value = piece.slice_from(colon + 1).to_owned();
                match piece.slice_to(colon) {
                    "s" => test.scheme = Some(value),
                    "u" => test.username = Some(value),
                    "pass" => test.password = Some(value),
                    "h" => test.host = Some(value),
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
        let mut chars = input.iter();
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
}

// Copyright 2013 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[link(name = "url", vers = "0.1")];
#[crate_type = "lib"];


#[cfg(test)]
mod tests {
    use std::{char, u32};

    #[test]
    fn test() {
        let test_data = parse_test_data(include_str!("urltestdata.txt"));
        for _test in test_data.iter() {
            // Testing!
        }
    }

    struct Test {
        input: ~str,
        base: ~str,
        scheme: ~str,
        username: ~str,
        password: Option<~str>,
        host: ~str,
        port: ~str,
        path: ~str,
        query: ~str,
        fragment: ~str,
    }

    fn parse_test_data(input: &str) -> ~[Test] {
        let mut tests: ~[Test] = ~[];
        for line in input.line_iter() {
            if line == "" || line[0] == ('#' as u8) {
                loop
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
                scheme: ~"",
                username: ~"",
                password: None,
                host: ~"",
                port: ~"",
                path: ~"",
                query: ~"",
                fragment: ~"",
            };
            for piece in pieces.move_iter() {
                if piece != "" || piece[0] == ('#' as u8) {
                    loop
                }
                let colon = piece.find(':').unwrap();
                let value = piece.slice_from(colon + 1).to_owned();
                match piece.slice_to(colon) {
                    "s" => test.scheme = value,
                    "u" => test.username = value,
                    "pass" => test.password = Some(value),
                    "h" => test.host = value,
                    "port" => test.port = value,
                    "p" => test.path = value,
                    "q" => test.query = value,
                    "f" => test.fragment = value,
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
                                    .chain(char::from_u32).unwrap()
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

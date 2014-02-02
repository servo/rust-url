// Copyright 2013 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


use std::u32;
use std::char;
use std::ascii::Ascii;


// Bootstring parameters for Punycode
static BASE: u32 = 36;
static T_MIN: u32 = 1;
static T_MAX: u32 = 26;
static SKEW: u32 = 38;
static DAMP: u32 = 700;
static INITIAL_BIAS: u32 = 72;
static INITIAL_N: u32 = 0x80;
static DELIMITER: char = '-';


#[inline]
fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
    delta /= if first_time { DAMP } else { 2 };
    delta += delta / num_points;
    let mut k = 0;
    while delta > ((BASE - T_MIN) * T_MAX) / 2 {
        delta /= BASE - T_MIN;
        k += BASE;
    }
    k + (((BASE - T_MIN + 1) * delta) / (delta + SKEW))
}


/// Convert Punycode to Unicode.
/// Return None on malformed input or overflow.
/// Overflow can only happen on inputs that take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
pub fn decode(input: &[Ascii]) -> Option<~[char]> {
    // Handle "basic" (ASCII) code points.
    // They are encoded as-is befor the last delimiter, if any.
    let (mut output, input) = match input.rposition_elem(&DELIMITER.to_ascii()) {
        None => (~[], input),
        Some(position) => (
            input.slice_to(position).map(|a| a.to_char()),
            if position > 0 { input.slice_from(position + 1) } else { input }
        )
    };
    let mut code_point = INITIAL_N;
    let mut bias = INITIAL_BIAS;
    let mut i = 0;
    let mut iter = input.iter();
    loop {
        let previous_i = i;
        let mut weight = 1;
        let mut k = BASE;
        let mut ascii = match iter.next() {
            None => break,
            Some(ascii) => ascii,
        };
        // Decode a generalized variable-length integer into delta,
        // which gets added to i.
        loop {
            let digit = match ascii.to_byte() {
                byte @ 0x30 .. 0x39 => byte - 0x30 + 26,  // 0..9
                byte @ 0x41 .. 0x5A => byte - 0x41,  // A..Z
                byte @ 0x61 .. 0x7A => byte - 0x61,  // a..z
                _ => return None
            } as u32;
            if digit > (u32::max_value - i) / weight {
                return None  // Overflow
            }
            i += digit * weight;
            let t = if k <= bias { T_MIN }
                    else if k >= bias + T_MAX { T_MAX }
                    else { k - bias };
            if digit < t {
                break
            }
            if weight > u32::max_value / (BASE - t) {
                return None  // Overflow
            }
            weight *= BASE - t;
            k += BASE;
            ascii = match iter.next() {
                None => return None,  // End of input before the end of this delta
                Some(ascii) => ascii,
            };
        }
        let length = output.len() as u32;
        bias = adapt(i - previous_i, length + 1, previous_i == 0);
        if i / (length + 1) > u32::max_value - code_point {
            return None  // Overflow
        }
        // i was supposed to wrap around from length+1 to 0,
        // incrementing code_point each time.
        code_point += i / (length + 1);
        i %= length + 1;
        let c = match char::from_u32(code_point) {
            Some(c) => c,
            None => return None
        };
        output.insert(i as uint, c);
        i += 1;
    }
    Some(output)
}


/// Convert Unicode to Punycode.
/// Return None on overflow, which can only happen on inputs that would take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
pub fn encode(input: &[char]) -> Option<~[Ascii]> {
    // Handle "basic" (ASCII) code points. They are encoded as-is.
    let mut output = input.iter().filter_map(|&c|
        if c.is_ascii() { Some(unsafe { c.to_ascii_nocheck() }) }
        else { None }
    ).to_owned_vec();
    let basic_length = output.len() as u32;
    if basic_length > 0 {
        output.push('-'.to_ascii())
    }
    let mut code_point = INITIAL_N;
    let mut delta = 0;
    let mut bias = INITIAL_BIAS;
    let mut processed = basic_length;
    let input_length = input.len() as u32;
    while processed < input_length {
        // All code points < code_point have been handled already.
        // Find the next larger one.
        let min_code_point = input.iter().map(|&c| c as u32)
                                  .filter(|&c| c >= code_point).min().unwrap();
        if min_code_point - code_point > (u32::max_value - delta) / (processed + 1) {
            return None  // Overflow
        }
        // Increase delta to advance the decoder’s <code_point,i> state to <min_code_point,0>
        delta += (min_code_point - code_point) * (processed + 1);
        code_point = min_code_point;
        for &c in input.iter() {
            let c = c as u32;
            if c < code_point {
                delta += 1;
                if delta == 0 {
                    return None  // Overflow
                }
            }
            if c == code_point {
                // Represent delta as a generalized variable-length integer:
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias { T_MIN }
                            else if k >= bias + T_MAX { T_MAX }
                            else { k - bias };
                    if q < t {
                        break
                    }
                    let value = t + ((q - t) % (BASE - t));
                    output.push(value_to_digit(value));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                output.push(value_to_digit(q));
                bias = adapt(delta, processed + 1, processed == basic_length);
                delta = 0;
                processed += 1;
            }
        }
        delta += 1;
        code_point += 1;
    }
    Some(output)
}


#[inline]
fn value_to_digit(value: u32) -> Ascii {
    let code_point = match value {
        0 .. 25 => value + 0x61,  // a..z
        26 .. 35 => value - 26 + 0x30,  // 0..9
        _ => fail!()
    };
    unsafe { (code_point as u8).to_ascii_nocheck() }
}


#[cfg(test)]
mod tests {
    use super::{decode, encode};
    use std::ascii::AsciiCast;
    use std::str::from_chars;
    use extra::json::{from_str, List, Object, String};

    fn one_test(description: &str, decoded: &str, encoded: &str) {
        match decode(encoded.to_ascii()) {
            None => fail!("Decoding {:?} failed.", encoded),
            Some(result) => {
                let result = from_chars(result);
                assert!(result.as_slice() == decoded,
                        format!("Incorrect decoding of {:?}:\n   {:?}\n!= {:?}\n{}",
                                encoded, result.as_slice(), decoded, description))
            }
        }

        match encode(decoded.chars().to_owned_vec()) {
            None => fail!("Encoding {:?} failed.", decoded),
            Some(result) => {
                let result = result.as_str_ascii();
                assert!(result.as_slice() == encoded,
                        format!("Incorrect encoding of {:?}:\n   {:?}\n!= {:?}\n{}",
                                decoded, result.as_slice(), encoded, description))
            }
        }
    }

    fn get_string<'a>(map: &'a ~Object, key: &~str) -> &'a str {
        match map.find(key) {
            Some(&String(ref s)) => s.as_slice(),
            None => "",
            _ => fail!(),
        }
    }

    #[test]
    fn test_punycode() {

        match from_str(include_str!("punycode_tests.json")) {
            Ok(List(tests)) => for test in tests.iter() {
                match test {
                    &Object(ref o) => one_test(
                        get_string(o, &~"description"),
                        get_string(o, &~"decoded"),
                        get_string(o, &~"encoded")
                    ),
                    _ => fail!(),
                }
            },
            other => fail!("{:?}", other)
        }
    }
}

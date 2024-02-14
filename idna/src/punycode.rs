// Copyright 2013 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Punycode ([RFC 3492](http://tools.ietf.org/html/rfc3492)) implementation.
//!
//! Since Punycode fundamentally works on unicode code points,
//! `encode` and `decode` take and return slices and vectors of `char`.
//! `encode_str` and `decode_to_string` provide convenience wrappers
//! that convert from and to Rust’s UTF-8 based `str` and `String` types.

use alloc::{string::String, vec::Vec};
use core::char;
use core::fmt::Write;
use core::u32;

// Bootstring parameters for Punycode
const BASE: u32 = 36;
const T_MIN: u32 = 1;
const T_MAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const INITIAL_BIAS: u32 = 72;
const INITIAL_N: u32 = 0x80;

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

/// Convert Punycode to an Unicode `String`.
///
/// Return None on malformed input or overflow.
/// Overflow can only happen on inputs that take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
#[inline]
pub fn decode_to_string(input: &str) -> Option<String> {
    Some(Decoder::default().decode(input.as_bytes()).ok()?.collect())
}

/// Convert Punycode to Unicode.
///
/// Return None on malformed input or overflow.
/// Overflow can only happen on inputs that take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
pub fn decode(input: &str) -> Option<Vec<char>> {
    Some(Decoder::default().decode(input.as_bytes()).ok()?.collect())
}

pub(crate) trait PunycodeCodeUnit {
    fn is_delimiter(&self) -> bool;
    fn is_ascii(&self) -> bool;
    fn digit(&self) -> Option<u32>;
    fn char(&self) -> char;
}

impl PunycodeCodeUnit for u8 {
    fn is_delimiter(&self) -> bool {
        *self == b'-'
    }
    fn is_ascii(&self) -> bool {
        // XXX how to skip this for internal caller while keeping this for external API?
        *self < 0x80
    }
    fn digit(&self) -> Option<u32> {
        let byte = *self;
        Some(match byte {
            byte @ b'0'..=b'9' => byte - b'0' + 26,
            byte @ b'A'..=b'Z' => byte - b'A',
            byte @ b'a'..=b'z' => byte - b'a',
            _ => return None,
        } as u32)
    }
    fn char(&self) -> char {
        // XXX changes externally-visible behavior of the Punycode API
        char::from(self.to_ascii_lowercase())
    }
}

impl PunycodeCodeUnit for char {
    fn is_delimiter(&self) -> bool {
        *self == '-'
    }
    fn is_ascii(&self) -> bool {
        // XXX should this just be `true` if we don't accept `&[char]` via
        // public API?
        true
        // *self < '\u{80}'
    }
    fn digit(&self) -> Option<u32> {
        let byte = *self;
        Some(match byte {
            byte @ '0'..='9' => u32::from(byte) - u32::from('0') + 26,
            // byte @ 'A'..='Z' => u32::from(byte) - u32::from('A'), // XXX not needed if no public input
            byte @ 'a'..='z' => u32::from(byte) - u32::from('a'),
            _ => return None,
        })
    }
    fn char(&self) -> char {
        *self
    }
}

#[derive(Default)]
pub(crate) struct Decoder {
    insertions: smallvec::SmallVec<[(usize, char); 59]>,
}

impl Decoder {
    /// Split the input iterator and return a Vec with insertions of encoded characters
    ///
    /// XXX: Add a policy parameter to skip overflow checks
    pub(crate) fn decode<'a, T: PunycodeCodeUnit + Copy>(
        &'a mut self,
        input: &'a [T],
    ) -> Result<Decode<'a, T>, ()> {
        self.insertions.clear();
        // Handle "basic" (ASCII) code points.
        // They are encoded as-is before the last delimiter, if any.
        let (base, input) = if let Some(position) = input.iter().rposition(|c| c.is_delimiter()) {
            (
                &input[..position],
                if position > 0 {
                    &input[position + 1..]
                } else {
                    input
                },
            )
        } else {
            (&input[..0], input)
        };

        if !base.iter().all(|c| c.is_ascii()) {
            return Err(());
        }

        let base_len = base.len();
        let mut length = base_len as u32;
        let mut code_point = INITIAL_N;
        let mut bias = INITIAL_BIAS;
        let mut i = 0;
        let mut iter = input.iter();
        loop {
            let previous_i = i;
            let mut weight = 1;
            let mut k = BASE;
            let mut byte = match iter.next() {
                None => break,
                Some(byte) => byte,
            };

            // Decode a generalized variable-length integer into delta,
            // which gets added to i.
            loop {
                let digit = if let Some(digit) = byte.digit() {
                    digit
                } else {
                    return Err(());
                };
                if digit > (u32::MAX - i) / weight {
                    return Err(()); // Overflow
                }
                i += digit * weight;
                let t = if k <= bias {
                    T_MIN
                } else if k >= bias + T_MAX {
                    T_MAX
                } else {
                    k - bias
                };
                if digit < t {
                    break;
                }
                if weight > u32::MAX / (BASE - t) {
                    return Err(()); // Overflow
                }
                weight *= BASE - t;
                k += BASE;
                byte = match iter.next() {
                    None => return Err(()), // End of input before the end of this delta
                    Some(byte) => byte,
                };
            }

            bias = adapt(i - previous_i, length + 1, previous_i == 0);
            if i / (length + 1) > u32::MAX - code_point {
                return Err(()); // Overflow
            }

            // i was supposed to wrap around from length+1 to 0,
            // incrementing code_point each time.
            code_point += i / (length + 1);
            i %= length + 1;
            let c = match char::from_u32(code_point) {
                Some(c) => c,
                None => return Err(()),
            };

            // Move earlier insertions farther out in the string
            for (idx, _) in &mut self.insertions {
                if *idx >= i as usize {
                    *idx += 1;
                }
            }
            self.insertions.push((i as usize, c));
            length += 1;
            i += 1;
        }

        self.insertions.sort_by_key(|(i, _)| *i);
        Ok(Decode {
            base: base.iter(),
            insertions: &self.insertions,
            inserted: 0,
            position: 0,
            len: base_len + self.insertions.len(),
        })
    }
}

pub(crate) struct Decode<'a, T>
where
    T: PunycodeCodeUnit + Copy,
{
    base: core::slice::Iter<'a, T>,
    pub(crate) insertions: &'a [(usize, char)],
    inserted: usize,
    position: usize,
    len: usize,
}

impl<'a, T: PunycodeCodeUnit + Copy> Iterator for Decode<'a, T> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.insertions.get(self.inserted) {
                Some((pos, c)) if *pos == self.position => {
                    self.inserted += 1;
                    self.position += 1;
                    return Some(*c);
                }
                _ => {}
            }
            if let Some(c) = self.base.next() {
                self.position += 1;
                return Some(c.char());
            } else if self.inserted >= self.insertions.len() {
                return None;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len - self.position;
        (len, Some(len))
    }
}

impl<'a, T: PunycodeCodeUnit + Copy> ExactSizeIterator for Decode<'a, T> {
    fn len(&self) -> usize {
        self.len - self.position
    }
}

/// Convert an Unicode `str` to Punycode.
///
/// This is a convenience wrapper around `encode`.
#[inline]
pub fn encode_str(input: &str) -> Option<String> {
    if input.len() > u32::MAX as usize {
        return None;
    }
    let mut buf = String::with_capacity(input.len());
    encode_into(input.chars(), &mut buf).ok().map(|()| buf)
}

/// Convert Unicode to Punycode.
///
/// Return None on overflow, which can only happen on inputs that would take more than
/// 63 encoded bytes, the DNS limit on domain name labels.
pub fn encode(input: &[char]) -> Option<String> {
    if input.len() > u32::MAX as usize {
        return None;
    }
    let mut buf = String::with_capacity(input.len());
    encode_into(input.iter().copied(), &mut buf)
        .ok()
        .map(|()| buf)
}

pub(crate) enum PunycodeEncodeError {
    Overflow,
    Sink,
}

impl From<core::fmt::Error> for PunycodeEncodeError {
    fn from(_: core::fmt::Error) -> Self {
        PunycodeEncodeError::Sink
    }
}

/// XXX: Add a policy parameter to skip overflow checks
pub(crate) fn encode_into<I, W>(input: I, output: &mut W) -> Result<(), PunycodeEncodeError>
where
    I: Iterator<Item = char> + Clone,
    W: Write + ?Sized,
{
    // Handle "basic" (ASCII) code points. They are encoded as-is.
    let (mut input_length, mut basic_length) = (0u32, 0);
    for c in input.clone() {
        input_length = input_length
            .checked_add(1)
            .ok_or(PunycodeEncodeError::Overflow)?;
        if c.is_ascii() {
            output.write_char(c)?;
            basic_length += 1;
        }
    }

    if basic_length > 0 {
        output.write_char('-')?;
    }
    let mut code_point = INITIAL_N;
    let mut delta = 0;
    let mut bias = INITIAL_BIAS;
    let mut processed = basic_length;
    while processed < input_length {
        // All code points < code_point have been handled already.
        // Find the next larger one.
        let min_code_point = input
            .clone()
            .map(|c| c as u32)
            .filter(|&c| c >= code_point)
            .min()
            .unwrap();
        if min_code_point - code_point > (u32::MAX - delta) / (processed + 1) {
            return Err(PunycodeEncodeError::Overflow); // Overflow
        }
        // Increase delta to advance the decoder’s <code_point,i> state to <min_code_point,0>
        delta += (min_code_point - code_point) * (processed + 1);
        code_point = min_code_point;
        for c in input.clone() {
            let c = c as u32;
            if c < code_point {
                delta = delta.checked_add(1).ok_or(PunycodeEncodeError::Overflow)?;
            }
            if c == code_point {
                // Represent delta as a generalized variable-length integer:
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        T_MIN
                    } else if k >= bias + T_MAX {
                        T_MAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    let value = t + ((q - t) % (BASE - t));
                    output.write_char(value_to_digit(value))?;
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                output.write_char(value_to_digit(q))?;
                bias = adapt(delta, processed + 1, processed == basic_length);
                delta = 0;
                processed += 1;
            }
        }
        delta += 1;
        code_point += 1;
    }
    Ok(())
}

#[inline]
fn value_to_digit(value: u32) -> char {
    match value {
        0..=25 => (value as u8 + b'a') as char,       // a..z
        26..=35 => (value as u8 - 26 + b'0') as char, // 0..9
        _ => panic!(),
    }
}

#[test]
#[ignore = "slow"]
#[cfg(target_pointer_width = "64")]
fn huge_encode() {
    let mut buf = String::new();
    assert!(encode_into(std::iter::repeat('ß').take(u32::MAX as usize + 1), &mut buf).is_err());
    assert_eq!(buf.len(), 0);
}

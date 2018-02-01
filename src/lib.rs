#[macro_use] extern crate matches;
pub extern crate mime;

use std::io;

pub enum DataUrlError {
    NotADataUrl,
    NoComma,
}

pub struct DataUrl<'a> {
    mime_type: mime::Mime,
    base64: bool,
    encoded_body_plus_fragment: &'a str,
}

impl<'a> DataUrl<'a> {
    /// <https://fetch.spec.whatwg.org/#data-url-processor>
    /// but starting from a string rather than a Url, to avoid extra string copies.
    pub fn process(input: &'a str) -> Result<Self, DataUrlError> {
        use DataUrlError::*;

        let after_colon = pretend_parse_data_url(input).ok_or(NotADataUrl)?;
        let comma = after_colon.bytes().position(|byte| byte == b',').ok_or(NoComma)?;

        let (mime_type, base64) = parse_header(&after_colon[..comma]);
        let encoded_body_plus_fragment = &after_colon[comma + 1..];

        Ok(DataUrl { mime_type, base64, encoded_body_plus_fragment })
    }

    pub fn mime_type(&self) -> &mime::Mime {
        &self.mime_type
    }

    /// Streaming-decode the data URL’s body to `sink`.
    ///
    /// Errors while writing to the sink are propagated.
    /// Invalid base64 causes an error with `e.kind() == ErrorKind::InvalidData`.
    /// When decoding without error, the URL’s fragment identifier is returned if it has one.
    ///
    /// The fragment identifier is represented as in the origin input.
    /// It needs to be either percent-encoded to obtain the same string as in a parsed URL,
    /// or percent-decoded to interpret it as text.
    pub fn decode_body<W>(&self, sink: W) -> io::Result<Option<&'a str>>
        where W: io::Write
    {
        if self.base64 {
            decode_with_base64(self.encoded_body_plus_fragment, sink)
        } else {
            decode_without_base64(self.encoded_body_plus_fragment, sink)
        }
    }

    pub fn decode_body_to_vec(&self) -> Result<(Vec<u8>, Option<&str>), ()> {
        let mut sink = Vec::new();
        match self.decode_body(&mut sink) {
            Ok(url_fragment) => {
                Ok((sink, url_fragment))
            }
            Err(e) => {
                // Vec::write_all never returns an error
                debug_assert!(e.kind() == io::ErrorKind::InvalidData && self.base64);

                Err(())
            }
        }
    }
}

macro_rules! require {
    ($condition: expr) => {
        if !$condition {
            return None
        }
    }
}

/// Similar to <https://url.spec.whatwg.org/#concept-basic-url-parser>
/// followed by <https://url.spec.whatwg.org/#concept-url-serializer>
///
/// * `None`: not a data URL.
///
/// * `Some(s)`: sort of the result of serialization, except:
///
///   - `data:` prefix removed
///   - The fragment is included
///   - Other components are **not** UTF-8 percent-encoded
///   - ASCII tabs and newlines in the middle are **not** removed
fn pretend_parse_data_url(input: &str) -> Option<&str> {
    // Trim C0 control or space
    let left_trimmed = input.trim_left_matches(|ch| ch <= ' ');

    let mut bytes = left_trimmed.bytes();
    {
        // Ignore ASCII tabs or newlines
        let mut iter = bytes.by_ref().filter(|&byte| !matches!(byte, b'\t' | b'\n' | b'\r'));
        require!(iter.next()?.to_ascii_lowercase() == b'd');
        require!(iter.next()?.to_ascii_lowercase() == b'a');
        require!(iter.next()?.to_ascii_lowercase() == b't');
        require!(iter.next()?.to_ascii_lowercase() == b'a');
        require!(iter.next()? == b':');
    }
    let bytes_consumed = left_trimmed.len() - bytes.len();
    let after_colon = &left_trimmed[bytes_consumed..];

    // Trim C0 control or space
    Some(after_colon.trim_right_matches(|ch| ch <= ' '))
}

fn parse_header(from_colon_to_comma: &str) -> (mime::Mime, bool) {
    let input = from_colon_to_comma.chars()
        .filter(|&c| !matches!(c, '\t' | '\n' | '\r'))  // Removed by the URL parser
        .collect::<String>();

    let input = input.trim_matches(' ');

    let (input, base64) = match without_base64_suffix(input) {
        Some(s) => (s, true),
        None => (input, false),
    };

    // FIXME: does Mime::from_str match the MIME Sniffing Standard’s parsing algorithm?
    // <https://mimesniff.spec.whatwg.org/#parse-a-mime-type>
    let mime_type = input.parse()
        .unwrap_or_else(|_| "text/plain;charset=US-ASCII".parse().unwrap());

    (mime_type, base64)
}

/// None: no base64 suffix
fn without_base64_suffix(s: &str) -> Option<&str> {
    remove_suffix(
        remove_suffix(s, "base64", str::eq_ignore_ascii_case)?
            .trim_right_matches(' '),
        ";", str::eq
    )
}

fn remove_suffix<'a, Eq>(haystack: &'a str, needle: &str, eq: Eq) -> Option<&'a str>
    where Eq: Fn(&str, &str) -> bool
{
    let start_index = haystack.len().checked_sub(needle.len())?;
    let (before, after) = haystack.split_at(start_index);
    if eq(after, needle) {
        Some(before)
    } else {
        None
    }
}

/// This is <https://url.spec.whatwg.org/#string-percent-decode> while also:
///
/// * Ignoring ASCII tab or newlines
/// * Stopping at the first '#' (which indicates the start of the fragment)
///
/// Anything that would have been UTF-8 percent-encoded by the URL parser
/// would be percent-decoded here.
/// We skip that round-trip and pass it through unchanged.
fn decode_without_base64<W>(encoded_body_plus_fragment: &str, mut sink: W)
                            -> io::Result<Option<&str>>
    where W: io::Write
{
    let bytes = encoded_body_plus_fragment.as_bytes();
    let mut slice_start = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        // We only need to look for 5 different "special" byte values.
        // For everything else we make slices as large as possible, borrowing the input,
        // in order to make fewer write_all() calls.
        if matches!(byte, b'%' | b'#' | b'\t' | b'\n' | b'\r') {
            // Write everything (if anything) "non-special" we’ve accumulated
            // before this special byte
            if i > slice_start {
                sink.write_all(&bytes[slice_start..i])?;
            }
            // Then deal with the special byte.
            match byte {
                b'%' => {
                    let l = bytes.get(i + 2).and_then(|&b| (b as char).to_digit(16));
                    let h = bytes.get(i + 1).and_then(|&b| (b as char).to_digit(16));
                    if let (Some(h), Some(l)) = (h, l) {
                        // '%' followed by two ASCII hex digits
                        let one_byte = h as u8 * 0x10 + l as u8;
                        sink.write_all(&[one_byte])?;
                        slice_start = i + 3;
                    } else {
                        // Do nothing. Leave slice_start unchanged.
                        // The % sign will be part of the next slice.
                    }
                }

                b'#' => {
                    let fragment_start = i + 1;
                    return Ok(Some(&encoded_body_plus_fragment[fragment_start..]))
                }

                // Ignore over '\t' | '\n' | '\r'
                _ => slice_start = i + 1
            }
        }
    }
    sink.write_all(&bytes[slice_start..])?;
    Ok(None)
}

/// `decode_without_base64()` composed with
/// <https://infra.spec.whatwg.org/#isomorphic-decode> composed with
/// <https://infra.spec.whatwg.org/#forgiving-base64-decode>.
fn decode_with_base64<W>(encoded_body_plus_fragment: &str, sink: W) -> io::Result<Option<&str>>
    where W: io::Write
{
    let mut decoder = Base64Decoder {
        sink,
        bit_buffer: 0,
        buffer_bit_length: 0,
        padding_symbols: 0,
    };
    let fragment = decode_without_base64(encoded_body_plus_fragment, &mut decoder)?;
    match (decoder.buffer_bit_length, decoder.padding_symbols) {
        (0, 0) => {
            // A multiple of four of alphabet symbols, and nothing else.
        }
        (12, 2) | (12, 0) => {
            // A multiple of four of alphabet symbols, followed by two more symbols,
            // optionally followed by two padding characters (which make a total multiple of four).
            let byte_buffer = [
                (decoder.bit_buffer >> 4) as u8,
            ];
            decoder.sink.write_all(&byte_buffer)?;
        }
        (18, 1) | (18, 0) => {
            // A multiple of four of alphabet symbols, followed by three more symbols,
            // optionally followed by one padding character (which make a total multiple of four).
            let byte_buffer = [
                (decoder.bit_buffer >> 10) as u8,
                (decoder.bit_buffer >> 2) as u8,
            ];
            decoder.sink.write_all(&byte_buffer)?;
        }
        _ => {
            // No other combination is acceptable
            Err(io::ErrorKind::InvalidData)?
        }
    }
    Ok(fragment)
}

struct Base64Decoder<W> {
    sink: W,
    bit_buffer: u32,
    buffer_bit_length: u8,
    padding_symbols: u8,
}

impl<W> io::Write for Base64Decoder<W> where W: io::Write {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> { unimplemented!() }
    fn flush(&mut self) -> io::Result<()> { unimplemented!() }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        for &byte in buf.iter() {
            let value = BASE64_DECODE_TABLE[byte as usize];
            if value < 0 {
                // A character that’s not part of the alphabet

                // Remove ASCII whitespace
                // '\t' | '\n' | '\r' was already filtered by decode_without_base64()
                if byte == b' ' || byte == b'\x0C' {
                    continue
                }

                if byte == b'=' {
                    self.padding_symbols = self.padding_symbols.saturating_add(8);
                    continue
                }

                Err(io::ErrorKind::InvalidData)?
            }
            if self.padding_symbols > 0 {
                // Alphabet symbols after padding
                Err(io::ErrorKind::InvalidData)?
            }
            self.bit_buffer <<= 6;
            self.bit_buffer |= value as u32;
            if self.buffer_bit_length < 24 {
                self.buffer_bit_length += 6;
            } else {
                // We’ve accumulated four times 6 bits, which equals three times 8 bits.
                let byte_buffer = [
                    (self.bit_buffer >> 16) as u8,
                    (self.bit_buffer >> 8) as u8,
                    self.bit_buffer as u8,
                ];
                self.sink.write_all(&byte_buffer)?;
                self.buffer_bit_length = 0;
                // No need to reset self.bit_buffer,
                // since next time we’re only gonna read relevant bits.
            }
        }
        Ok(())
    }
}

/// Generated by `make_base64_decode_table.py` based on "Table 1: The Base 64 Alphabet"
/// at <https://tools.ietf.org/html/rfc4648#section-4>
///
/// Array indices are the byte value of symbols.
/// Array values are their positions in the base64 alphabet,
/// or -1 for symbols not in the alphabet.
/// The position contributes 6 bits to the decoded bytes.
const BASE64_DECODE_TABLE: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1, -1, 63,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1,
    -1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14,
    15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1,
    -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
    41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

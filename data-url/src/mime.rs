use std::fmt::{self, Write};
use std::str::FromStr;

/// <https://mimesniff.spec.whatwg.org/#mime-type-representation>
#[derive(Debug, PartialEq, Eq)]
pub struct Mime {
    pub type_: String,
    pub subtype: String,
    /// (name, value)
    pub parameters: Vec<(String, String)>,
}

impl Mime {
    pub fn get_parameter<P>(&self, name: &P) -> Option<&str>
    where
        P: ?Sized + PartialEq<str>,
    {
        self.parameters
            .iter()
            .find(|&&(ref n, _)| name == &**n)
            .map(|&(_, ref v)| &**v)
    }
}

#[derive(Debug)]
pub struct MimeParsingError(());

/// <https://mimesniff.spec.whatwg.org/#parsing-a-mime-type>
impl FromStr for Mime {
    type Err = MimeParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).ok_or(MimeParsingError(()))
    }
}

fn parse(s: &str) -> Option<Mime> {
    let trimmed = s.trim_matches(ascii_whitespace);

    let (type_, rest) = split2(trimmed, '/');
    require!(only_http_token_code_points(type_) && !type_.is_empty());

    let (subtype, rest) = split2(rest?, ';');
    let subtype = subtype.trim_end_matches(ascii_whitespace);
    require!(only_http_token_code_points(subtype) && !subtype.is_empty());

    let mut parameters = Vec::new();
    if let Some(rest) = rest {
        parse_parameters(rest, &mut parameters)
    }

    Some(Mime {
        type_: type_.to_ascii_lowercase(),
        subtype: subtype.to_ascii_lowercase(),
        parameters,
    })
}

fn split2(s: &str, separator: char) -> (&str, Option<&str>) {
    let mut iter = s.splitn(2, separator);
    let first = iter.next().unwrap();
    (first, iter.next())
}

#[allow(clippy::manual_strip)] // introduced in 1.45, MSRV is 1.36
fn parse_parameters(s: &str, parameters: &mut Vec<(String, String)>) {
    let mut semicolon_separated = s.split(';');

    while let Some(piece) = semicolon_separated.next() {
        let piece = piece.trim_start_matches(ascii_whitespace);
        let (name, value) = split2(piece, '=');
        if name.is_empty() || !only_http_token_code_points(name) || contains(&parameters, name) {
            continue;
        }
        if let Some(value) = value {
            let value = if value.starts_with('"') {
                let max_len = value.len().saturating_sub(2); // without start or end quotes
                let mut unescaped_value = String::with_capacity(max_len);
                let mut chars = value[1..].chars();
                'until_closing_quote: loop {
                    while let Some(c) = chars.next() {
                        match c {
                            '"' => break 'until_closing_quote,
                            '\\' => unescaped_value.push(chars.next().unwrap_or('\\')),
                            _ => unescaped_value.push(c),
                        }
                    }
                    if let Some(piece) = semicolon_separated.next() {
                        // A semicolon inside a quoted value is not a separator
                        // for the next parameter, but part of the value.
                        unescaped_value.push(';');
                        chars = piece.chars()
                    } else {
                        break;
                    }
                }
                if !valid_value(&unescaped_value) {
                    continue;
                }
                unescaped_value
            } else {
                let value = value.trim_end_matches(ascii_whitespace);
                if !valid_value(value) {
                    continue;
                }
                value.to_owned()
            };
            parameters.push((name.to_ascii_lowercase(), value))
        }
    }
}

fn contains(parameters: &[(String, String)], name: &str) -> bool {
    parameters.iter().any(|&(ref n, _)| n == name)
}

fn valid_value(s: &str) -> bool {
    s.chars().all(|c| {
        // <https://mimesniff.spec.whatwg.org/#http-quoted-string-token-code-point>
        matches!(c, '\t' | ' '..='~' | '\u{80}'..='\u{FF}')
    }) && !s.is_empty()
}

/// <https://mimesniff.spec.whatwg.org/#serializing-a-mime-type>
impl fmt::Display for Mime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_)?;
        f.write_str("/")?;
        f.write_str(&self.subtype)?;
        for &(ref name, ref value) in &self.parameters {
            f.write_str(";")?;
            f.write_str(name)?;
            f.write_str("=")?;
            if only_http_token_code_points(value) {
                f.write_str(value)?
            } else {
                f.write_str("\"")?;
                for c in value.chars() {
                    if c == '"' || c == '\\' {
                        f.write_str("\\")?
                    }
                    f.write_char(c)?
                }
                f.write_str("\"")?
            }
        }
        Ok(())
    }
}

fn ascii_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

fn only_http_token_code_points(s: &str) -> bool {
    s.bytes().all(|byte| IS_HTTP_TOKEN[byte as usize])
}

macro_rules! byte_map {
    ($($flag:expr,)*) => ([
        $($flag != 0,)*
    ])
}

// Copied from https://github.com/hyperium/mime/blob/v0.3.5/src/parse.rs#L293
#[rustfmt::skip]
static IS_HTTP_TOKEN: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

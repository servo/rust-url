// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! These methods are not meant for use in Rust code,
//! only to help implement the JavaScript URLUtils API: http://url.spec.whatwg.org/#urlutils

use super::{Url, UrlParser, SchemeType, SchemeData, RelativeSchemeData};
use parser::{ParseError, ParseResult, Context};
use percent_encoding::{utf8_percent_encode_to, USERNAME_ENCODE_SET, PASSWORD_ENCODE_SET};


#[allow(dead_code)]
pub struct UrlUtilsWrapper<'a> {
    pub url: &'a mut Url,
    pub parser: &'a UrlParser<'a>,
}

#[doc(hidden)]
pub trait UrlUtils {
    fn set_scheme(&mut self, input: &str) -> ParseResult<()>;
    fn set_username(&mut self, input: &str) -> ParseResult<()>;
    fn set_password(&mut self, input: &str) -> ParseResult<()>;
    fn set_host_and_port(&mut self, input: &str) -> ParseResult<()>;
    fn set_host(&mut self, input: &str) -> ParseResult<()>;
    fn set_port(&mut self, input: &str) -> ParseResult<()>;
    fn set_path(&mut self, input: &str) -> ParseResult<()>;
    fn set_query(&mut self, input: &str) -> ParseResult<()>;
    fn set_fragment(&mut self, input: &str) -> ParseResult<()>;
}

impl<'a> UrlUtils for UrlUtilsWrapper<'a> {
    /// `URLUtils.protocol` setter
    fn set_scheme(&mut self, input: &str) -> ParseResult<()> {
        match ::parser::parse_scheme(input, Context::Setter) {
            Some((scheme, _)) => {
                if self.parser.get_scheme_type(&self.url.scheme).same_as(self.parser.get_scheme_type(&scheme)) {
                    return Err(ParseError::InvalidScheme);
                }
                self.url.scheme = scheme;
                Ok(())
            },
            None => Err(ParseError::InvalidScheme),
        }
    }

    /// `URLUtils.username` setter
    fn set_username(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            SchemeData::Relative(RelativeSchemeData { ref mut username, .. }) => {
                username.truncate(0);
                utf8_percent_encode_to(input, USERNAME_ENCODE_SET, username);
                Ok(())
            },
            SchemeData::NonRelative(_) => Err(ParseError::CannotSetUsernameWithNonRelativeScheme)
        }
    }

    /// `URLUtils.password` setter
    fn set_password(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            SchemeData::Relative(RelativeSchemeData { ref mut password, .. }) => {
                if input.len() == 0 {
                    *password = None;
                    return Ok(());
                }
                let mut new_password = String::new();
                utf8_percent_encode_to(input, PASSWORD_ENCODE_SET, &mut new_password);
                *password = Some(new_password);
                Ok(())
            },
            SchemeData::NonRelative(_) => Err(ParseError::CannotSetPasswordWithNonRelativeScheme)
        }
    }

    /// `URLUtils.host` setter
    fn set_host_and_port(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            SchemeData::Relative(RelativeSchemeData {
                ref mut host, ref mut port, ref mut default_port, ..
            }) => {
                let scheme_type = self.parser.get_scheme_type(&self.url.scheme);
                let (new_host, new_port, new_default_port, _) = try!(::parser::parse_host(
                    input, scheme_type, self.parser));
                *host = new_host;
                *port = new_port;
                *default_port = new_default_port;
                Ok(())
            },
            SchemeData::NonRelative(_) => Err(ParseError::CannotSetHostPortWithNonRelativeScheme)
        }
    }

    /// `URLUtils.hostname` setter
    fn set_host(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            SchemeData::Relative(RelativeSchemeData { ref mut host, .. }) => {
                let (new_host, _) = try!(::parser::parse_hostname(input, self.parser));
                *host = new_host;
                Ok(())
            },
            SchemeData::NonRelative(_) => Err(ParseError::CannotSetHostWithNonRelativeScheme)
        }
    }

    /// `URLUtils.port` setter
    fn set_port(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            SchemeData::Relative(RelativeSchemeData { ref mut port, ref mut default_port, .. }) => {
                let scheme_type = self.parser.get_scheme_type(&self.url.scheme);
                if scheme_type == SchemeType::FileLike {
                    return Err(ParseError::CannotSetPortWithFileLikeScheme);
                }
                let (new_port, new_default_port, _) = try!(::parser::parse_port(
                    input, scheme_type, self.parser));
                *port = new_port;
                *default_port = new_default_port;
                Ok(())
            },
            SchemeData::NonRelative(_) => Err(ParseError::CannotSetPortWithNonRelativeScheme)
        }
    }

    /// `URLUtils.pathname` setter
    fn set_path(&mut self, input: &str) -> ParseResult<()> {
        match self.url.scheme_data {
            SchemeData::Relative(RelativeSchemeData { ref mut path, .. }) => {
                let scheme_type = self.parser.get_scheme_type(&self.url.scheme);
                let (new_path, _) = try!(::parser::parse_path_start(
                    input, Context::Setter, scheme_type, self.parser));
                *path = new_path;
                Ok(())
            },
            SchemeData::NonRelative(_) => Err(ParseError::CannotSetPathWithNonRelativeScheme)
        }
    }

    /// `URLUtils.search` setter
    fn set_query(&mut self, input: &str) -> ParseResult<()> {
        self.url.query = if input.is_empty() {
            None
        } else {
            let input = if input.starts_with("?") { &input[1..] } else { input };
            let (new_query, _) = try!(::parser::parse_query(
                input, Context::Setter, self.parser));
            Some(new_query)
        };
        Ok(())
    }

    /// `URLUtils.hash` setter
    fn set_fragment(&mut self, input: &str) -> ParseResult<()> {
        if self.url.scheme == "javascript" {
            return Err(ParseError::CannotSetJavascriptFragment)
        }
        self.url.fragment = if input.is_empty() {
            None
        } else {
            let input = if input.starts_with("#") { &input[1..] } else { input };
            Some(try!(::parser::parse_fragment(input, self.parser)))
        };
        Ok(())
    }
}

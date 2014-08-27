// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Serialize various pieces of URLs to writers and string.

use std::fmt;
use std::io;
use std::io::{IoResult, Writer};
use super::{Url, SchemeData, RelativeSchemeData, NonRelativeSchemeData};
use host::{Host, Domain, Ipv6, write_ipv6_address};


pub trait TextWriter {
    fn write_str(&mut self, string: &str) -> IoResult<()>;
}

impl TextWriter for String {
    #[inline]
    fn write_str(&mut self, string: &str) -> IoResult<()> {
        self.push_str(string);
        Ok(())
    }
}

impl<'a> TextWriter for fmt::Formatter<'a> {
    #[inline]
    fn write_str(&mut self, string: &str) -> IoResult<()> {
        self.write(string.as_bytes()).map_err(|_| io::standard_error(io::OtherIoError))
    }
}


macro_rules! build_string {
    ($function: expr $( , $arg: expr )*) => {
        {
            let mut string = String::new();
            $function(&mut string $( , $arg )*).unwrap();
            string
        }
    }
}


pub fn write_url<W: TextWriter>(writer: &mut W, url: &Url) -> IoResult<()> {
    try!(write_url_no_fragment(writer, url));
    match url.fragment {
        None => {},
        Some(ref fragment) => {
            try!(writer.write_str("#"));
            try!(writer.write_str(fragment.as_slice()));
        }
    }
    Ok(())
}

pub fn write_url_no_fragment<W: TextWriter>(writer: &mut W, url: &Url) -> IoResult<()> {
    try!(writer.write_str(url.scheme.as_slice()));
    try!(writer.write_str(":"));
    try!(write_scheme_data(writer, &url.scheme_data));
    match url.query {
        None => (),
        Some(ref query) => {
            try!(writer.write_str("?"));
            try!(writer.write_str(query.as_slice()));
        }
    }
    Ok(())
}

pub fn write_scheme_data<W: TextWriter>(writer: &mut W, scheme_data: &SchemeData) -> IoResult<()> {
    match *scheme_data {
        RelativeSchemeData(ref scheme_data) => write_relative_scheme_data(writer, scheme_data),
        NonRelativeSchemeData(ref scheme_data) => writer.write_str(scheme_data.as_slice()),
    }
}

pub fn write_relative_scheme_data<W: TextWriter>(writer: &mut W, scheme_data: &RelativeSchemeData)
                                             -> IoResult<()> {
    try!(writer.write_str("//"));
    try!(write_authority(
        writer,
        scheme_data.username.as_slice(),
        scheme_data.password.as_ref().map(|s| s.as_slice()),
        &scheme_data.host,
        scheme_data.port));
    write_path(writer, scheme_data.path.as_slice())
}

pub fn write_authority<W: TextWriter>(writer: &mut W, username: &str, password: Option<&str>,
                                  host: &Host, port: Option<u16>)
                                  -> IoResult<()> {
    try!(write_userinfo(writer, username, password));
    write_authority_no_userinfo(writer, host, port)
}

pub fn write_userinfo<W: TextWriter>(writer: &mut W, username: &str, password: Option<&str>)
                                 -> IoResult<()> {
    if !username.is_empty() || password.is_some() {
        try!(writer.write_str(username.as_slice()));
        match password {
            None => (),
            Some(password) => {
                try!(writer.write_str(":"));
                try!(writer.write_str(password.as_slice()));
            }
        }
        try!(writer.write_str("@"));
    }
    Ok(())
}

pub fn write_authority_no_userinfo<W: TextWriter>(writer: &mut W, host: &Host, port: Option<u16>)
                                              -> IoResult<()> {
    try!(write_host(writer, host));
    match port {
        Some(port) => {
            try!(writer.write_str(":"));
            // FIXME: Avoid allocating here. (Requires Unicode-based formatting.)
            try!(writer.write_str(port.to_string().as_slice()));
        },
        None => {}
    }
    Ok(())
}

pub fn write_host<W: TextWriter>(writer: &mut W, host: &Host) -> IoResult<()> {
    match *host {
        Domain(ref domain) => writer.write_str(domain.as_slice()),
        Ipv6(ref address) => {
            try!(writer.write_str("["));
            try!(write_ipv6_address(writer, address));
            writer.write_str("]")
        }
    }
}

pub fn write_path<S: Str, W: TextWriter>(writer: &mut W, path: &[S]) -> IoResult<()> {
    if path.is_empty() {
        writer.write_str("/")
    } else {
        for path_part in path.iter() {
            try!(writer.write_str("/"));
            try!(writer.write_str(path_part.as_slice()));
        }
        Ok(())
    }
}



/// Formatting Tests
#[cfg(test)]
mod tests {
    use Url;
    use super::{write_path, write_userinfo};

    #[test]
    fn path_formatting() {
        let data = [
            (vec![], "/"),
            (vec![""], "/"),
            (vec!["test", "path"], "/test/path"),
            (vec!["test", "path", ""], "/test/path/")
        ];
        for &(ref path, result) in data.iter() {
            assert_eq!(build_string!(write_path, path.as_slice()), result.to_string());
        }
    }

    #[test]
    fn userinfo_formatting() {
        // Test data as (username, password, result) tuples.
        let data = [
            ("", None, ""),
            ("", Some(""), ":@"),
            ("", Some("password"), ":password@"),
            ("username", None, "username@"),
            ("username", Some(""), "username:@"),
            ("username", Some("password"), "username:password@")
        ];
        for &(username, password, result) in data.iter() {
            assert_eq!(build_string!(write_userinfo, username, password), result.to_string());
        }
    }

    #[test]
    fn relative_scheme_url_formatting() {
        let data = [
            ("http://example.com/", "http://example.com/"),
            ("http://addslash.com", "http://addslash.com/"),
            ("http://@emptyuser.com/", "http://emptyuser.com/"),
            ("http://:@emptypass.com/", "http://:@emptypass.com/"),
            ("http://user@user.com/", "http://user@user.com/"),
            ("http://user:pass@userpass.com/", "http://user:pass@userpass.com/"),
            ("http://slashquery.com/path/?q=something", "http://slashquery.com/path/?q=something"),
            ("http://noslashquery.com/path?q=something", "http://noslashquery.com/path?q=something")
        ];
        for &(input, result) in data.iter() {
            let url = Url::parse(input).unwrap();
            assert_eq!(url.to_string(), result.to_string());
        }
    }
}

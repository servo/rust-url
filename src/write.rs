// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Serialize various pieces of URLs to writers and string.
//!
//! These writers can be used to coerce various URL parts into strings.
//!
//! You can use `<writer>.to_string()`, as the writers implement `Show`.

use std::io::{MemWriter, Writer, IoResult};
use super::{Url, SchemeData, RelativeSchemeData, NonRelativeSchemeData};
use host::{Host, Domain, Ipv6, write_ipv6_address};


pub fn build_string(build: |&mut MemWriter| -> IoResult<()>) -> String {
    let mut writer = MemWriter::new();
    build(&mut writer).unwrap();
    String::from_utf8(writer.unwrap()).unwrap()
}


pub fn write_url<W: Writer>(url: &Url, writer: &mut W) -> IoResult<()> {
    try!(write_url_no_fragment(url, writer));
    match url.fragment {
        None => {},
        Some(ref fragment) => {
            try!(writer.write(b"#"));
            try!(writer.write(fragment.as_bytes()));
        }
    }
    Ok(())
}

pub fn write_url_no_fragment<W: Writer>(url: &Url, writer: &mut W) -> IoResult<()> {
    try!(writer.write(url.scheme.as_bytes()));
    try!(writer.write(b":"));
    try!(write_scheme_data(&url.scheme_data, writer));
    match url.query {
        None => (),
        Some(ref query) => {
            try!(writer.write(b"?"));
            try!(writer.write(query.as_bytes()));
        }
    }
    Ok(())
}

pub fn write_scheme_data<W: Writer>(scheme_data: &SchemeData, writer: &mut W) -> IoResult<()> {
    match *scheme_data {
        RelativeSchemeData(ref scheme_data) => write_relative_scheme_data(scheme_data, writer),
        NonRelativeSchemeData(ref scheme_data) => writer.write(scheme_data.as_bytes()),
    }
}

pub fn write_relative_scheme_data<W: Writer>(scheme_data: &RelativeSchemeData, writer: &mut W)
                                             -> IoResult<()> {
    try!(writer.write(b"//"));
    try!(write_authority(
        scheme_data.username.as_slice(),
        scheme_data.password.as_ref().map(|s| s.as_slice()),
        &scheme_data.host,
        scheme_data.port,
        writer));
    write_path(scheme_data.path.as_slice(), writer)
}

pub fn write_authority<W: Writer>(username: &str, password: Option<&str>,
                                  host: &Host, port: Option<u16>, writer: &mut W)
                                  -> IoResult<()> {
    try!(write_userinfo(username, password, writer));
    write_authority_no_userinfo(host, port, writer)
}

pub fn write_userinfo<W: Writer>(username: &str, password: Option<&str>, writer: &mut W)
                                 -> IoResult<()> {
    if !username.is_empty() || password.is_some() {
        try!(writer.write(username.as_bytes()));
        match password {
            None => (),
            Some(password) => {
                try!(writer.write(b":"));
                try!(writer.write(password.as_bytes()));
            }
        }
        try!(writer.write(b"@"));
    }
    Ok(())
}

pub fn write_authority_no_userinfo<W: Writer>(host: &Host, port: Option<u16>, writer: &mut W)
                                              -> IoResult<()> {
    try!(write_host(host, writer));
    match port {
        Some(port) => {
            try!(writer.write(b":"));
            try!(write!(writer, "{}", port));
        },
        None => {}
    }
    Ok(())
}

pub fn write_host<W: Writer>(host: &Host, writer: &mut W) -> IoResult<()> {
    match *host {
        Domain(ref domain) => writer.write(domain.as_bytes()),
        Ipv6(ref address) => {
            try!(writer.write(b"["));
            try!(write_ipv6_address(address, writer));
            writer.write(b"]")
        }
    }
}

pub fn write_path<S: Str, W: Writer>(path: &[S], writer: &mut W) -> IoResult<()> {
    if path.is_empty() {
        writer.write(b"/")
    } else {
        for path_part in path.iter() {
            try!(writer.write(b"/"));
            try!(writer.write(path_part.as_slice().as_bytes()));
        }
        Ok(())
    }
}



/// Formatting Tests
#[cfg(test)]
mod tests {
    use Url;
    use super::{write_path, write_userinfo, build_string};

    #[test]
    fn path_formatting() {
        let data = [
            (vec![], "/"),
            (vec![""], "/"),
            (vec!["test", "path"], "/test/path"),
            (vec!["test", "path", ""], "/test/path/")
        ];
        for &(ref path, result) in data.iter() {
            assert_eq!(build_string(|writer| {
                write_path(path.as_slice(), writer)
            }), result.to_string());
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
            assert_eq!(build_string(|writer| {
                write_userinfo(username, password, writer)
            }), result.to_string());
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

// Copyright 2013-2014 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate url;

use std::net::{Ipv4Addr, Ipv6Addr};
use url::{Host, Url};

#[test]
fn new_file_paths() {
    use std::path::{Path, PathBuf};
    if cfg!(unix) {
        assert_eq!(Url::from_file_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new("../relative")), Err(()));
    } else {
        assert_eq!(Url::from_file_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new(r"..\relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new(r"\drive-relative")), Err(()));
        assert_eq!(Url::from_file_path(Path::new(r"\\ucn\")), Err(()));
    }

    if cfg!(unix) {
        let mut url = Url::from_file_path(Path::new("/foo/bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["foo".to_string(), "bar".to_string()][..]));
        assert!(url.to_file_path() == Ok(PathBuf::from("/foo/bar")));

        url.path_mut().unwrap()[1] = "ba\0r".to_string();
        url.to_file_path().is_ok();

        url.path_mut().unwrap()[1] = "ba%00r".to_string();
        url.to_file_path().is_ok();
    }
}

#[test]
#[cfg(unix)]
fn new_path_bad_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::prelude::*;
    use std::path::{Path, PathBuf};

    let url = Url::from_file_path(Path::new("/foo/ba%80r")).unwrap();
    let os_str = OsStr::from_bytes(b"/foo/ba\x80r");
    assert_eq!(url.to_file_path(), Ok(PathBuf::from(os_str)));
}

#[test]
fn new_path_windows_fun() {
    if cfg!(windows) {
        use std::path::{Path, PathBuf};
        let mut url = Url::from_file_path(Path::new(r"C:\foo\bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["C:".to_string(), "foo".to_string(), "bar".to_string()][..]));
        assert_eq!(url.to_file_path(),
                   Ok(PathBuf::from(r"C:\foo\bar")));

        url.path_mut().unwrap()[2] = "ba\0r".to_string();
        assert!(url.to_file_path().is_ok());

        url.path_mut().unwrap()[2] = "ba%00r".to_string();
        assert!(url.to_file_path().is_ok());

        // Invalid UTF-8
        url.path_mut().unwrap()[2] = "ba%80r".to_string();
        assert!(url.to_file_path().is_err());
    }
}


#[test]
fn new_directory_paths() {
    use std::path::Path;

    if cfg!(unix) {
        assert_eq!(Url::from_directory_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new("../relative")), Err(()));

        let url = Url::from_directory_path(Path::new("/foo/bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["foo".to_string(), "bar".to_string(),
                                      "".to_string()][..]));
    } else {
        assert_eq!(Url::from_directory_path(Path::new("relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new(r"..\relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new(r"\drive-relative")), Err(()));
        assert_eq!(Url::from_directory_path(Path::new(r"\\ucn\")), Err(()));

        let url = Url::from_directory_path(Path::new(r"C:\foo\bar")).unwrap();
        assert_eq!(url.host(), Some(&Host::Domain("".to_string())));
        assert_eq!(url.path(), Some(&["C:".to_string(), "foo".to_string(),
                                      "bar".to_string(), "".to_string()][..]));
    }
}

#[test]
fn from_str() {
    assert!("http://testing.com/this".parse::<Url>().is_ok());
}

#[test]
fn issue_124() {
    let url: Url = "file:a".parse().unwrap();
    assert_eq!(url.path().unwrap(), ["a"]);
    let url: Url = "file:...".parse().unwrap();
    assert_eq!(url.path().unwrap(), ["..."]);
    let url: Url = "file:..".parse().unwrap();
    assert_eq!(url.path().unwrap(), [""]);
}

#[test]
fn relative_scheme_data_equality() {
    use std::hash::{Hash, Hasher, SipHasher};

    fn check_eq(a: &Url, b: &Url) {
        assert_eq!(a, b);

        let mut h1 = SipHasher::new();
        a.hash(&mut h1);
        let mut h2 = SipHasher::new();
        b.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    fn url(s: &str) -> Url {
        let rv = s.parse().unwrap();
        check_eq(&rv, &rv);
        rv
    }

    // Doesn't care if default port is given.
    let a: Url = url("https://example.com/");
    let b: Url = url("https://example.com:443/");
    check_eq(&a, &b);

    // Different ports
    let a: Url = url("http://example.com/");
    let b: Url = url("http://example.com:8080/");
    assert!(a != b);

    // Different scheme
    let a: Url = url("http://example.com/");
    let b: Url = url("https://example.com/");
    assert!(a != b);

    // Different host
    let a: Url = url("http://foo.com/");
    let b: Url = url("http://bar.com/");
    assert!(a != b);

    // Missing path, automatically substituted. Semantically the same.
    let a: Url = url("http://foo.com");
    let b: Url = url("http://foo.com/");
    check_eq(&a, &b);
}

#[test]
fn host() {
    let a = Host::parse("www.mozilla.org").unwrap();
    let b = Host::parse("1.35.33.49").unwrap();
    let c = Host::parse("[2001:0db8:85a3:08d3:1319:8a2e:0370:7344]").unwrap();
    let d = Host::parse("1.35.+33.49").unwrap();
    assert_eq!(a, Host::Domain("www.mozilla.org".to_owned()));
    assert_eq!(b, Host::Ipv4(Ipv4Addr::new(1, 35, 33, 49)));
    assert_eq!(c, Host::Ipv6(Ipv6Addr::new(0x2001, 0x0db8, 0x85a3, 0x08d3,
        0x1319, 0x8a2e, 0x0370, 0x7344)));
    assert_eq!(d, Host::Domain("1.35.+33.49".to_owned()));
    assert_eq!(Host::parse("[::]").unwrap(), Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)));
    assert_eq!(Host::parse("[::1]").unwrap(), Host::Ipv6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));
    assert_eq!(Host::parse("0x1.0X23.0x21.061").unwrap(), Host::Ipv4(Ipv4Addr::new(1, 35, 33, 49)));
    assert_eq!(Host::parse("0x1232131").unwrap(), Host::Ipv4(Ipv4Addr::new(1, 35, 33, 49)));
    assert!(Host::parse("42.0x1232131").is_err());
    assert_eq!(Host::parse("111").unwrap(), Host::Ipv4(Ipv4Addr::new(0, 0, 0, 111)));
    assert_eq!(Host::parse("2..2.3").unwrap(), Host::Domain("2..2.3".to_owned()));
    assert!(Host::parse("192.168.0.257").is_err());
}

#[test]
fn test_idna() {
    assert!("http://goșu.ro".parse::<Url>().is_ok());
    assert_eq!(Url::parse("http://☃.net/").unwrap().domain(), Some("xn--n3h.net"));
}

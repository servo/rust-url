extern crate url;

use url::{Url, Host};
use url::format::{PathFormatter, UserInfoFormatter};

#[test]
fn path_formatting() {
    let data = [
        (vec![], "/"),
        (vec![""], "/"),
        (vec!["test", "path"], "/test/path"),
        (vec!["test", "path", ""], "/test/path/")
    ];
    for &(ref path, result) in &data {
        assert_eq!(PathFormatter {
            path: path
        }.to_string(), result.to_string());
    }
}

#[test]
fn host() {
    // libstd’s `Display for Ipv6Addr` serializes 0:0:0:0:0:0:_:_ and 0:0:0:0:0:ffff:_:_
    // using IPv4-like syntax, as suggested in https://tools.ietf.org/html/rfc5952#section-4
    // but https://url.spec.whatwg.org/#concept-ipv6-serializer specifies not to.

    // Not [::0.0.0.2] / [::ffff:0.0.0.2]
    assert_eq!(Host::parse("[0::2]").unwrap().to_string(), "[::2]");
    assert_eq!(Host::parse("[0::ffff:0:2]").unwrap().to_string(), "[::ffff:0:2]");
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
    for &(username, password, result) in &data {
        assert_eq!(UserInfoFormatter {
            username: username,
            password: password
        }.to_string(), result.to_string());
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
    for &(input, result) in &data {
        let url = Url::parse(input).unwrap();
        assert_eq!(url.to_string(), result.to_string());
    }
}

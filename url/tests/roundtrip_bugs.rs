// Reproduction tests for bugs #1101 and #1102
use url::Url;

#[test]
fn test_bug_1101_file_url_roundtrip_with_host() {
    // Bug #1101: file:// URL parse roundtrip mismatch
    // When parsing file URLs with both host and path components,
    // the path normalization was stripping semantic leading slashes,
    // causing roundtrip failures
    let input = "file://.cRe!+aacRddddddddddddddtpe=//t:/a|et/!..";
    let url1 = Url::parse(input).unwrap();
    let serialized = url1.to_string();
    let url2 = Url::parse(&serialized).unwrap();

    assert_eq!(url1.host_str(), url2.host_str(), "Host should match after roundtrip");
    assert_eq!(url1.path(), url2.path(), "Path should match after roundtrip");
    assert_eq!(url1, url2, "Full URL should roundtrip correctly");
}

#[test]
fn test_bug_1102_set_host_localhost_roundtrip() {
    // Bug #1102: set_host("localhost") on file:// URLs doesn't normalize
    // The parser normalizes "localhost" to empty host per WHATWG spec,
    // but set_host() was not applying the same normalization
    let mut url = Url::parse("file:///path").unwrap();
    url.set_host(Some("localhost")).unwrap();
    let serialized = url.to_string();
    let reparsed = Url::parse(&serialized).unwrap();

    assert_eq!(url.host_str(), reparsed.host_str(), "Host should match after set_host roundtrip");
    assert_eq!(url, reparsed, "URL should roundtrip correctly after set_host(localhost)");
}

#[test]
fn test_file_url_localhost_normalization() {
    // Additional test: verify that "localhost" is normalized to empty host
    // for file:// URLs per WHATWG spec
    let url1 = Url::parse("file://localhost/path").unwrap();
    let url2 = Url::parse("file:///path").unwrap();

    assert_eq!(url1.host_str(), url2.host_str(), "localhost should normalize to empty host");
    assert_eq!(url1, url2, "file://localhost/path should equal file:///path");
}

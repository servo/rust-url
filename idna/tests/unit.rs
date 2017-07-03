use unicode_normalization::char::is_combining_mark;

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_with_length_check() {
    let config = idna::Config::default()
        .verify_dns_length(true)
        .check_hyphens(true)
        .use_std3_ascii_rules(true);

    assert!(config.to_ascii("xn--").is_err());
    assert!(config.to_ascii("xn---").is_err());
    assert!(config.to_ascii("xn-----").is_err());
    assert!(config.to_ascii("xn--.").is_err());
    assert!(config.to_ascii("xn--...").is_err());
    assert!(config.to_ascii(".xn--").is_err());
    assert!(config.to_ascii("...xn--").is_err());
    assert!(config.to_ascii("xn--.xn--").is_err());
    assert!(config.to_ascii("xn--.example.org").is_err());
}

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_without_length_check() {
    let config = idna::Config::default()
        .verify_dns_length(false)
        .check_hyphens(true)
        .use_std3_ascii_rules(true);

    assert_eq!(config.to_ascii("xn--").unwrap(), "");
    assert!(config.to_ascii("xn---").is_err());
    assert!(config.to_ascii("xn-----").is_err());
    assert_eq!(config.to_ascii("xn--.").unwrap(), ".");
    assert_eq!(config.to_ascii("xn--...").unwrap(), "...");
    assert_eq!(config.to_ascii(".xn--").unwrap(), ".");
    assert_eq!(config.to_ascii("...xn--").unwrap(), "...");
    assert_eq!(config.to_ascii("xn--.xn--").unwrap(), ".");
    assert_eq!(config.to_ascii("xn--.example.org").unwrap(), ".example.org");
}

#[test]
fn test_v5() {
    let config = idna::Config::default()
        .verify_dns_length(true)
        .use_std3_ascii_rules(true);

    // IdnaTest:784 蔏｡𑰺
    assert!(is_combining_mark('\u{11C3A}'));
    assert!(config.to_ascii("\u{11C3A}").is_err());
    assert!(config.to_ascii("\u{850f}.\u{11C3A}").is_err());
    assert!(config.to_ascii("\u{850f}\u{ff61}\u{11C3A}").is_err());
}

#[test]
fn test_v8_bidi_rules() {
    let config = idna::Config::default()
        .verify_dns_length(true)
        .use_std3_ascii_rules(true);

    assert_eq!(config.to_ascii("abc").unwrap(), "abc");
    assert_eq!(config.to_ascii("123").unwrap(), "123");
    assert_eq!(config.to_ascii("אבּג").unwrap(), "xn--kdb3bdf");
    assert_eq!(config.to_ascii("ابج").unwrap(), "xn--mgbcm");
    assert_eq!(config.to_ascii("abc.ابج").unwrap(), "abc.xn--mgbcm");
    assert_eq!(config.to_ascii("אבּג.ابج").unwrap(), "xn--kdb3bdf.xn--mgbcm");

    // Bidi domain names cannot start with digits
    assert!(config.to_ascii("0a.\u{05D0}").is_err());
    assert!(config.to_ascii("0à.\u{05D0}").is_err());

    // Bidi chars may be punycode-encoded
    assert!(config.to_ascii("xn--0ca24w").is_err());
}

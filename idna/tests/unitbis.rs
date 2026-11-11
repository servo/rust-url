use idna::uts46::AsciiDenyList;
use idna::uts46::DnsLength;
use idna::uts46::Hyphens;

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_with_length_check() {
    let config = idna::uts46::Uts46::new();

    assert!(config
        .to_ascii(
            b"xn--",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn---",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn-----",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--.",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--...",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            b".xn--",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"...xn--",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--.xn--",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--.example.org",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
}

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_without_length_check() {
    let config = idna::uts46::Uts46::new();

    assert!(config
        .to_ascii(
            b"xn--",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn---",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn-----",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--.",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--...",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b".xn--",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"...xn--",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--.xn--",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
    assert!(config
        .to_ascii(
            b"xn--.example.org",
            AsciiDenyList::URL,
            Hyphens::Allow,
            DnsLength::Ignore
        )
        .is_err());
}
/*
// http://www.unicode.org/reports/tr46/#Table_Example_Processing
#[test]
fn test_examples() {
    let codec = idna::uts46bis::Uts46::new();
    let mut out = String::new();

    assert_matches!(codec.to_unicode("Bloß.de", &mut out), Ok(()));
    assert_eq!(out, "bloß.de");

    out.clear();
    assert_matches!(codec.to_unicode("xn--blo-7ka.de", &mut out), Ok(()));
    assert_eq!(out, "bloß.de");

    out.clear();
    assert_matches!(codec.to_unicode("u\u{308}.com", &mut out), Ok(()));
    assert_eq!(out, "ü.com");

    out.clear();
    assert_matches!(codec.to_unicode("xn--tda.com", &mut out), Ok(()));
    assert_eq!(out, "ü.com");

    out.clear();
    assert_matches!(codec.to_unicode("xn--u-ccb.com", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("a⒈com", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("xn--a-ecp.ru", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("xn--0.pt", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("日本語。ＪＰ", &mut out), Ok(()));
    assert_eq!(out, "日本語.jp");

    out.clear();
    assert_matches!(codec.to_unicode("☕.us", &mut out), Ok(()));
    assert_eq!(out, "☕.us");
}
*/

#[test]
fn test_v5() {
    let config = idna::uts46::Uts46::new();

    // IdnaTest:784 蔏｡𑰺
    assert!(config
        .to_ascii(
            "\u{11C3A}".as_bytes(),
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            "\u{850f}.\u{11C3A}".as_bytes(),
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            "\u{850f}\u{ff61}\u{11C3A}".as_bytes(),
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
}

#[test]
fn test_v8_bidi_rules() {
    let config = idna::uts46::Uts46::new();

    assert_eq!(
        config
            .to_ascii(
                b"abc",
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "abc"
    );
    assert_eq!(
        config
            .to_ascii(
                b"123",
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "123"
    );
    assert_eq!(
        config
            .to_ascii(
                "אבּג".as_bytes(),
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "xn--kdb3bdf"
    );
    assert_eq!(
        config
            .to_ascii(
                "ابج".as_bytes(),
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "xn--mgbcm"
    );
    assert_eq!(
        config
            .to_ascii(
                "abc.ابج".as_bytes(),
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "abc.xn--mgbcm"
    );
    assert_eq!(
        config
            .to_ascii(
                "אבּג.ابج".as_bytes(),
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "xn--kdb3bdf.xn--mgbcm"
    );

    // Bidi domain names cannot start with digits
    assert!(config
        .to_ascii(
            "0a.\u{05D0}".as_bytes(),
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
    assert!(config
        .to_ascii(
            "0à.\u{05D0}".as_bytes(),
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());

    // Bidi chars may be punycode-encoded
    assert!(config
        .to_ascii(
            b"xn--0ca24w",
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
}

#[test]
fn emoji_domains() {
    // HOT BEVERAGE is allowed here...
    let config = idna::uts46::Uts46::new();
    assert_eq!(
        config
            .to_ascii(
                "☕.com".as_bytes(),
                AsciiDenyList::STD3,
                Hyphens::Check,
                DnsLength::Verify,
            )
            .unwrap(),
        "xn--53h.com"
    );
}

#[test]
fn unicode_before_delimiter() {
    let config = idna::uts46::Uts46::new();
    assert!(config
        .to_ascii(
            "xn--f\u{34a}-PTP".as_bytes(),
            AsciiDenyList::STD3,
            Hyphens::Check,
            DnsLength::Verify,
        )
        .is_err());
}

#[test]
fn upper_case_ascii_in_punycode() {
    let config = idna::uts46::Uts46::new();
    let (unicode, result) =
        config.to_unicode("xn--A-1ga".as_bytes(), AsciiDenyList::STD3, Hyphens::Check);
    assert!(result.is_ok());
    assert_eq!(&unicode, "aö");
}

/// https://github.com/whatwg/url/pull/914
#[test]
fn ascii_domain_that_fails_unicode_processing() {
    for (input, expected) in [
        ("xn--8i7caa.famitei.net", "xn--8i7caa.famitei.net"),
        ("XN--board-NGR.org", "xn--board-ngr.org"),
        ("XN--0.XN--NXA", "xn--0.xn--nxa"),
        ("xn--", "xn--"),
    ] {
        let ascii = idna::domain_to_ascii_cow(input.as_bytes(), AsciiDenyList::URL).unwrap();
        assert_eq!(ascii, expected);
        let (unicode, result) = idna::domain_to_unicode(input);
        assert!(result.is_ok());
        assert_eq!(unicode, expected);
    }

    // The deny list still applies.
    assert!(idna::domain_to_ascii_cow(b"xn--a<b", AsciiDenyList::URL).is_err());

    // Non-ASCII input with errors is still rejected by both.
    assert!(idna::domain_to_ascii_cow("a\u{200C}b".as_bytes(), AsciiDenyList::URL).is_err());
    assert!(idna::domain_to_unicode("a\u{200C}b").1.is_err());

    // Valid Punycode still decodes.
    assert_eq!(idna::domain_to_unicode("xn--fa-hia.de").0, "faß.de");
}

extern crate idna;
extern crate unicode_normalization;

use idna::uts46;
use unicode_normalization::char::is_combining_mark;


/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_with_length_check() {
    fn _to_ascii(domain: &str) -> Result<String, uts46::Errors> {
        uts46::to_ascii(
            domain,
            uts46::Flags {
                transitional_processing: false,
                use_std3_ascii_rules: true,
                verify_dns_length: true,
            },
        )
    }

    assert!(_to_ascii("xn--").is_err());
    assert!(_to_ascii("xn---").is_err());
    assert!(_to_ascii("xn-----").is_err());
    assert!(_to_ascii("xn--.").is_err());
    assert!(_to_ascii("xn--...").is_err());
    assert!(_to_ascii(".xn--").is_err());
    assert!(_to_ascii("...xn--").is_err());
    assert!(_to_ascii("xn--.xn--").is_err());
    assert!(_to_ascii("xn--.example.org").is_err());
}

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_without_length_check() {
    fn _to_ascii(domain: &str) -> Result<String, uts46::Errors> {
        uts46::to_ascii(
            domain,
            uts46::Flags {
                transitional_processing: false,
                use_std3_ascii_rules: true,
                verify_dns_length: false,
            },
        )
    }

    assert_eq!(_to_ascii("xn--"), Ok("".to_owned()));
    assert!(_to_ascii("xn---").is_err());
    assert!(_to_ascii("xn-----").is_err());
    assert_eq!(_to_ascii("xn--."), Ok(".".to_owned()));
    assert_eq!(_to_ascii("xn--..."), Ok("...".to_owned()));
    assert_eq!(_to_ascii(".xn--"), Ok(".".to_owned()));
    assert_eq!(_to_ascii("...xn--"), Ok("...".to_owned()));
    assert_eq!(_to_ascii("xn--.xn--"), Ok(".".to_owned()));
    assert_eq!(_to_ascii("xn--.example.org"), Ok(".example.org".to_owned()));
}

#[test]
fn test_v5() {
    fn _to_ascii(domain: &str) -> Result<String, uts46::Errors> {
        uts46::to_ascii(
            domain,
            uts46::Flags {
                transitional_processing: false,
                use_std3_ascii_rules: true,
                verify_dns_length: true,
            },
        )
    }

    // IdnaTest:784 蔏｡𑰺
    assert!(is_combining_mark('\u{11C3A}'));
    assert!(_to_ascii("\u{11C3A}").is_err());
    assert!(_to_ascii("\u{850f}.\u{11C3A}").is_err());
    assert!(_to_ascii("\u{850f}\u{ff61}\u{11C3A}").is_err());
}

#[test]
fn test_v8_bidi_rules() {
    fn _to_ascii(domain: &str) -> Result<String, uts46::Errors> {
        uts46::to_ascii(
            domain,
            uts46::Flags {
                transitional_processing: false,
                use_std3_ascii_rules: true,
                verify_dns_length: true,
            },
        )
    }

    assert_eq!(_to_ascii("abc"), Ok("abc".to_owned()));
    assert_eq!(_to_ascii("123"), Ok("123".to_owned()));
    assert_eq!(_to_ascii("אבּג"), Ok("xn--kdb3bdf".to_owned()));
    assert_eq!(_to_ascii("ابج"), Ok("xn--mgbcm".to_owned()));
    assert_eq!(_to_ascii("abc.ابج"), Ok("abc.xn--mgbcm".to_owned()));
    assert_eq!(
        _to_ascii("אבּג.ابج"),
        Ok("xn--kdb3bdf.xn--mgbcm".to_owned())
    );

    // Bidi domain names cannot start with digits
    assert!(_to_ascii("0a.\u{05D0}").is_err());
    assert!(_to_ascii("0à.\u{05D0}").is_err());

    // Bidi chars may be punycode-encoded
    assert!(_to_ascii("xn--0ca24w").is_err());
}

// Regression test for https://github.com/servo/rust-url/issues/1114
//
// A path segment shaped like a Windows drive letter ("a:") but NOT in drive
// position (path[0]) is an ordinary segment and must be popped by "..".
// Previously this panicked in debug builds (and returned the wrong path in
// release). A genuine drive letter in drive position must still be preserved.

use url::Url;

#[test]
fn issue_1114_join_pops_non_drive_position_segment() {
    let cases = [
        ("file:///w:/a:", "file:..", "file:///w:/"),
        ("file:///w:/a:", "..", "file:///w:/"),
        ("file:///C:/a:", "file:..", "file:///C:/"),
        // a drive letter in drive position ("a:" as path[0]) is preserved
        ("file:///a:/b", "..", "file:///a:/"),
        ("file:///a:/b/c", "..", "file:///a:/"),
        // a genuine sole drive letter is not over-popped
        ("file:///w:/", "..", "file:///w:/"),
        ("file:///w:", "..", "file:///w:/"),
        ("file:///a:", "..", "file:///a:/"),
    ];
    for (base, rel, expected) in &cases {
        let joined = Url::parse(base).unwrap().join(rel).unwrap();
        assert_eq!(joined.as_str(), *expected, "join: {base} + {rel}");
    }
}

#[test]
fn issue_1114_parse_keeps_drive_letter() {
    // ".." right after a drive letter must keep the drive letter
    let cases = [
        ("file:///a:/..", "file:///a:/"),
        ("file:///c:/..", "file:///c:/"),
        ("file:///a:/../x", "file:///a:/x"),
    ];
    for (input, expected) in &cases {
        assert_eq!(Url::parse(input).unwrap().as_str(), *expected, "parse: {input}");
    }
}

extern crate url;

use url::form_urlencoded::*;

#[test]
fn test_form_urlencoded() {
    let pairs = &[
        ("foo".to_string(), "é&".to_string()),
        ("bar".to_string(), "".to_string()),
        ("foo".to_string(), "#".to_string())
    ];
    let encoded = serialize(pairs);
    assert_eq!(encoded, "foo=%C3%A9%26&bar=&foo=%23");
    assert_eq!(parse(encoded.as_bytes()), pairs.to_vec());
}

#[test]
fn test_form_serialize() {
    let pairs = [("foo", "é&"),
                 ("bar", ""),
                 ("foo", "#")];

    let want = "foo=%C3%A9%26&bar=&foo=%23";
    // Works with referenced tuples
    assert_eq!(serialize(pairs.iter()), want);
    // Works with owned tuples
    assert_eq!(serialize(pairs.iter().map(|p| (p.0, p.1))), want);

}

extern crate url;
use url::{Url, Origin};

#[test]
fn test_origin_eq() {
    let a = Url::parse("http://example.org").unwrap();
    let b = Url::parse("http://mozilla.org").unwrap();
    assert!(a.origin() != b.origin());
    assert!(a.origin() == a.origin());
    let c = Url::parse("file:///home/user/foobar/Documents/letter.odf").unwrap();
    let d = Url::parse("file:///home/user/foobar/Images/holiday.png").unwrap();
    let c_origin = c.origin();
    assert!(c.origin() != d.origin());
    assert!(c.origin() != c.origin());
    assert!(c_origin != c_origin);
    assert!(c_origin != c_origin.clone());
    assert!(Origin::Opaque != Origin::Opaque);
}

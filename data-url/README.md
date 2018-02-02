# data-url

![crates.io](https://img.shields.io/crates/v/url.svg)
![docs.rs](https://docs.rs/data-url/)

Processing of `data:` URLs in Rust according to the Fetch Standard:
<https://fetch.spec.whatwg.org/#data-urls>
but starting from a string rather than a parsed URL to avoid extra copies.

```rust
use data_url::{DataUrl, mime};
//!
let url = DataUrl::process("data:,Hello%20World!").unwrap();
let (body, fragment) = url.decode_to_vec().unwrap();
//!
assert_eq!(url.mime_type().type_, "text");
assert_eq!(url.mime_type().subtype, "plain");
assert_eq!(url.mime_type().get_parameter("charset"), Some("US-ASCII"));
assert_eq!(body, b"Hello World!");
assert!(fragment.is_none());
```

// Copyright 2017 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*!

This crate provides wrappers and convenience functions to make rust-url
and Serde work hand in hand.

The supported types are:

* `url::Url`

# How do I use a data type with a `Url` member with Serde?

Use the serde attributes `deserialize_with` and `serialize_with`.

```
#[derive(serde::Serialize, serde::Deserialize)]
struct MyStruct {
    #[serde(serialize_with = "serialize")]
    url: Url,
}
```

# How do I use a data type with an unnamed `Url` member with serde?

Same problem, same solution.

```
#[derive(serde::Serialize, serde::Deserialize)]
enum MyEnum {
    A(#[serde(with = "url_serde")] Url, OtherType),
}
```

# How do I encode a `Url` value with `serde_json::to_string`?

Use the `Ser` wrapper.

```
serde_json::to_string(&Ser::new(&url))
```

# How do I decode a `Url` value with `serde_json::parse`?

Use the `De` wrapper.

```
serde_json::from_str(r"http:://www.rust-lang.org").map(De::into_inner)
```

# How do I send `Url` values as part of an IPC channel?

Use the `Serde` wrapper. It implements `Deref` and `DerefMut` for convenience.

```
ipc::channel::<Serde<Url>>()
```
*/

#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate serde;
#[cfg(test)] #[macro_use] extern crate serde_derive;
#[cfg(test)] extern crate serde_json;
extern crate url;

use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::cmp::PartialEq;
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::str;
use url::{Url, Host};

/// Serialises `value` with a given serializer.
///
/// This is useful to serialize `rust-url` types used in structure fields or
/// tuple members with `#[serde(serialize_with = "url_serde::serialize")]`.
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer, for<'a> Ser<'a, T>: Serialize
{
    Ser::new(value).serialize(serializer)
}

/// A wrapper to serialize `rust-url` types.
///
/// This is useful with functions such as `serde_json::to_string`.
///
/// Values of this type can only be passed to the `serde::Serialize` trait.
#[derive(Debug)]
pub struct Ser<'a, T: 'a>(&'a T);

impl<'a, T> Ser<'a, T> where Ser<'a, T>: Serialize {
    /// Returns a new `Ser` wrapper.
    #[inline(always)]
    pub fn new(value: &'a T) -> Self {
        Ser(value)
    }
}

/// Serializes this URL into a `serde` stream.
impl<'a> Serialize for Ser<'a, Url> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(self.0.as_str())
    }
}

/// Serializes this Option<URL> into a `serde` stream.
impl<'a> Serialize for Ser<'a, Option<Url>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        if let Some(url) = self.0.as_ref() {
            serializer.serialize_some(url.as_str())
        } else {
            serializer.serialize_none()
        }
    }
}

impl<'a, String> Serialize for Ser<'a, Host<String>> where String: AsRef<str> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match *self.0 {
            Host::Domain(ref s) => serializer.serialize_str(s.as_ref()),
            Host::Ipv4(_) | Host::Ipv6(_) => {
                // max("101.102.103.104".len(),
                //     "[1000:1002:1003:1004:1005:1006:101.102.103.104]".len())
                const MAX_LEN: usize = 47;
                let mut buffer = [0; MAX_LEN];
                serializer.serialize_str(display_into_buffer(&self.0, &mut buffer))
            }
        }
    }
}

/// Like .to_string(), but doesn’t allocate memory for a `String`.
///
/// Panics if `buffer` is too small.
fn display_into_buffer<'a, T: fmt::Display>(value: &T, buffer: &'a mut [u8]) -> &'a str {
    let remaining_len;
    {
        let mut remaining = &mut *buffer;
        write!(remaining, "{}", value).unwrap();
        remaining_len = remaining.len()
    }
    let written_len = buffer.len() - remaining_len;
    let written = &buffer[..written_len];

    // write! only provides std::fmt::Formatter to Display implementations,
    // which has methods write_str and write_char but no method to write arbitrary bytes.
    // Therefore, `written` is well-formed in UTF-8.
    #[allow(unsafe_code)]
    unsafe {
        str::from_utf8_unchecked(written)
    }
}

/// Deserialises a `T` value with a given deserializer.
///
/// This is useful to deserialize Url types used in structure fields or
/// tuple members with `#[serde(deserialize_with = "url_serde::deserialize")]`.
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where D: Deserializer<'de>, De<T>: Deserialize<'de>
{
    De::deserialize(deserializer).map(De::into_inner)
}

/// A wrapper to deserialize `rust-url` types.
///
/// This is useful with functions such as `serde_json::from_str`.
///
/// Values of this type can only be obtained through
/// the `serde::Deserialize` trait.
#[derive(Debug)]
pub struct De<T>(T);

impl<'de, T> De<T> where De<T>: serde::Deserialize<'de> {
    /// Consumes this wrapper, returning the deserialized value.
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Deserializes this URL from a `serde` stream.
impl<'de> Deserialize<'de> for De<Url> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let string_representation: String = Deserialize::deserialize(deserializer)?;
        Url::parse(&string_representation).map(De).map_err(|err| {
            serde::de::Error::custom(err.description())
        })
    }
}

/// Deserializes this Option<URL> from a `serde` stream.
impl<'de> Deserialize<'de> for De<Option<Url>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let option_representation: Option<String> = Deserialize::deserialize(deserializer)?;
        if let Some(s) = option_representation {
            return Url::parse(&s)
                .map(Some)
                .map(De)
                .map_err(|err| {serde::de::Error::custom(err.description())});
        }
        Ok(De(None))

    }
}

impl<'de> Deserialize<'de> for De<Host> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let string_representation: String = Deserialize::deserialize(deserializer)?;
        Host::parse(&string_representation).map(De).map_err(|err| {
            serde::de::Error::custom(err.description())
        })
    }
}

/// A convenience wrapper to be used as a type parameter, for example when
/// a `Vec<T>` or an `HashMap<K, V>` need to be passed to serde.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Serde<T>(pub T);

/// A convenience type alias for Serde<Url>.
pub type SerdeUrl = Serde<Url>;

impl<'de, T> Serde<T>
where De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    /// Consumes this wrapper, returning the inner value.
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'de, T> fmt::Debug for Serde<T>
where T: fmt::Debug, De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(formatter)
    }
}

impl<'de, T> Deref for Serde<T>
where De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<'de, T> DerefMut for Serde<T>
where De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<'de, T: PartialEq> PartialEq<T> for Serde<T>
where De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    fn eq(&self, other: &T) -> bool {
        self.0 == *other
    }
}

impl<'de, T> Deserialize<'de> for Serde<T>
where De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        De::deserialize(deserializer).map(De::into_inner).map(Serde)
    }
}

impl<'de, T> Serialize for Serde<T>
where De<T>: Deserialize<'de>, for<'a> Ser<'a, T>: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        Ser(&self.0).serialize(serializer)
    }
}

#[test]
fn test_ser_de_url() {
    let url = Url::parse("http://www.test.com/foo/bar?$param=bazz").unwrap();
    let s = serde_json::to_string(&Ser::new(&url)).unwrap();
    let new_url: Url = serde_json::from_str(&s).map(De::into_inner).unwrap();
    assert_eq!(url, new_url);
}

#[test]
fn test_derive_deserialize_with_for_url() {
    #[derive(Deserialize, Debug, Eq, PartialEq)]
    struct Test {
        #[serde(deserialize_with = "deserialize", rename = "_url_")]
        url: Url
    }

    let url_str = "http://www.test.com/foo/bar?$param=bazz";

    let expected = Test {
        url: Url::parse(url_str).unwrap()
    };
    let json_string = format!(r#"{{"_url_": "{}"}}"#, url_str);
    let got: Test = serde_json::from_str(&json_string).unwrap();
    assert_eq!(expected, got);

}

#[test]
fn test_derive_deserialize_with_for_option_url() {
    #[derive(Deserialize, Debug, Eq, PartialEq)]
    struct Test {
        #[serde(deserialize_with = "deserialize", rename = "_url_")]
        url: Option<Url>
    }

    let url_str = "http://www.test.com/foo/bar?$param=bazz";

    let expected = Test {
        url: Some(Url::parse(url_str).unwrap())
    };
    let json_string = format!(r#"{{"_url_": "{}"}}"#, url_str);
    let got: Test = serde_json::from_str(&json_string).unwrap();
    assert_eq!(expected, got);

    let expected = Test {
        url: None
    };
    let json_string = r#"{"_url_": null}"#;
    let got: Test = serde_json::from_str(&json_string).unwrap();
    assert_eq!(expected, got);
}

#[test]
fn test_derive_serialize_with_for_url() {
    #[derive(Serialize, Debug, Eq, PartialEq)]
    struct Test {
        #[serde(serialize_with = "serialize", rename = "_url_")]
        url: Url
    }

    let url_str = "http://www.test.com/foo/bar?$param=bazz";

    let expected = format!(r#"{{"_url_":"{}"}}"#, url_str);
    let input = Test {url: Url::parse(url_str).unwrap()};
    let got = serde_json::to_string(&input).unwrap();
    assert_eq!(expected, got);
}

#[test]
fn test_derive_serialize_with_for_option_url() {
    #[derive(Serialize, Debug, Eq, PartialEq)]
    struct Test {
        #[serde(serialize_with = "serialize", rename = "_url_")]
        url: Option<Url>
    }

    let url_str = "http://www.test.com/foo/bar?$param=bazz";

    let expected = format!(r#"{{"_url_":"{}"}}"#, url_str);
    let input = Test {url: Some(Url::parse(url_str).unwrap())};
    let got = serde_json::to_string(&input).unwrap();
    assert_eq!(expected, got);

    let expected = format!(r#"{{"_url_":null}}"#);
    let input = Test {url: None};
    let got = serde_json::to_string(&input).unwrap();
    assert_eq!(expected, got);
}

#[test]
fn test_derive_with_for_url() {
    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
    struct Test {
        #[serde(with = "self", rename = "_url_")]
        url: Url
    }

    let url_str = "http://www.test.com/foo/bar?$param=bazz";
    let json_string = format!(r#"{{"_url_":"{}"}}"#, url_str);

    // test deserialization
    let expected = Test {
        url: Url::parse(url_str).unwrap()
    };
    let got: Test = serde_json::from_str(&json_string).unwrap();
    assert_eq!(expected, got);

    // test serialization
    let input = Test {url: Url::parse(url_str).unwrap()};
    let got = serde_json::to_string(&input).unwrap();
    assert_eq!(json_string, got);
}

#[test]
fn test_host() {
    for host in &[
        Host::Domain("foo.com".to_owned()),
        Host::Ipv4("127.0.0.1".parse().unwrap()),
        Host::Ipv6("::1".parse().unwrap()),
    ] {
        let json = serde_json::to_string(&Ser(host)).unwrap();
        let de: De<Host> = serde_json::from_str(&json).unwrap();
        assert_eq!(de.into_inner(), *host)
    }
}

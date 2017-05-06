use Url;
use Host;
use host::HostInternal;

use std::error::Error;

use legacy_serde::{de, Serialize, Serializer, Deserialize, Deserializer};

/// Serializes this URL into a `serde` stream.
///
/// This implementation is only available if the `serde` Cargo feature is enabled.
impl Serialize for Url {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        serializer.serialize_str(self.as_str())
    }
}

/// Deserializes this URL from a `serde` stream.
///
/// This implementation is only available if the `serde` Cargo feature is enabled.
impl Deserialize for Url {
    fn deserialize<D>(deserializer: &mut D) -> Result<Url, D::Error> where D: Deserializer {
        let string_representation: String = try!(Deserialize::deserialize(deserializer));
        Url::parse(&string_representation).map_err(|err| {
            de::Error::invalid_value(err.description())
        })
    }
}

impl Serialize for HostInternal {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        // This doesn’t use `derive` because that involves
        // large dependencies (that take a long time to build), and
        // either Macros 1.1 which are not stable yet or a cumbersome build script.
        //
        // Implementing `Serializer` correctly for an enum is tricky,
        // so let’s use existing enums that already do.
        use std::net::IpAddr;
        match *self {
            HostInternal::None => None,
            HostInternal::Domain => Some(None),
            HostInternal::Ipv4(addr) => Some(Some(IpAddr::V4(addr))),
            HostInternal::Ipv6(addr) => Some(Some(IpAddr::V6(addr))),
        }.serialize(serializer)
    }
}

impl Deserialize for HostInternal {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        use std::net::IpAddr;
        Ok(match try!(Deserialize::deserialize(deserializer)) {
            None => HostInternal::None,
            Some(None) => HostInternal::Domain,
            Some(Some(IpAddr::V4(addr))) => HostInternal::Ipv4(addr),
            Some(Some(IpAddr::V6(addr))) => HostInternal::Ipv6(addr),
        })
    }
}

impl<S: Serialize>  Serialize for Host<S> {
    fn serialize<R>(&self, serializer: &mut R) -> Result<(), R::Error> where R: Serializer {
        use std::net::IpAddr;
        match *self {
            Host::Domain(ref s) => Ok(s),
            Host::Ipv4(addr) => Err(IpAddr::V4(addr)),
            Host::Ipv6(addr) => Err(IpAddr::V6(addr)),
        }.serialize(serializer)
    }
}

impl<S: Deserialize> Deserialize for Host<S> {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error> where D: Deserializer {
        use std::net::IpAddr;
        Ok(match try!(Deserialize::deserialize(deserializer)) {
            Ok(s) => Host::Domain(s),
            Err(IpAddr::V4(addr)) => Host::Ipv4(addr),
            Err(IpAddr::V6(addr)) => Host::Ipv6(addr),
        })
    }
}

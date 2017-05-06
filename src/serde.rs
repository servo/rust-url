use std::fmt::{self, Formatter};
use std::marker::PhantomData;

use Url;
use HostInternal;
use Host;

use serde1::ser::{Serialize, Serializer};
use serde1::de::{Deserialize, Deserializer, Visitor, EnumAccess, VariantAccess, Unexpected, Error};

impl Serialize for Url {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Url {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = try!(String::deserialize(deserializer));
        s.parse().map_err(Error::custom)
    }
}

// This is equivalent to:
//
//    #[derive(Serialize)]
//    enum HostInternal {
//        None,
//        Domain,
//        Ipv4(Ipv4Addr),
//        Ipv6(Ipv6Addr),
//    }
//
// The serde_derive generated code contains its own "extern crate serde" so we
// can't use it here because the crate is actually called serde1. These next
// four impls can be replaced with derive after dropping support for old serde.
impl Serialize for HostInternal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self {
            HostInternal::None => {
                serializer.serialize_unit_variant("HostInternal", 0, "None")
            }
            HostInternal::Domain => {
                serializer.serialize_unit_variant("HostInternal", 1, "Domain")
            }
            HostInternal::Ipv4(ref addr) => {
                serializer.serialize_newtype_variant("HostInternal", 2, "Ipv4", addr)
            }
            HostInternal::Ipv6(ref addr) => {
                serializer.serialize_newtype_variant("HostInternal", 3, "Ipv6", addr)
            }
        }
    }
}

// This is equivalent to:
//
//    #[derive(Deserialize)]
//    enum HostInternal {
//        None,
//        Domain,
//        Ipv4(Ipv4Addr),
//        Ipv6(Ipv6Addr),
//    }
impl<'de> Deserialize<'de> for HostInternal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        enum Variant {
            None,
            Domain,
            Ipv4,
            Ipv6,
        }

        struct VariantVisitor;

        // This is equivalent to:
        //
        //    #[derive(Deserialize)]
        //    #[serde(variant_identifier)]
        //    enum Variant {
        //        None,
        //        Domain,
        //        Ipv4,
        //        Ipv6,
        //    }
        impl<'de> Visitor<'de> for VariantVisitor {
            type Value = Variant;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("variant identifier")
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
                where E: Error
            {
                match v {
                    0 => Ok(Variant::None),
                    1 => Ok(Variant::Domain),
                    2 => Ok(Variant::Ipv4),
                    3 => Ok(Variant::Ipv6),
                    _ => {
                        Err(Error::invalid_value(Unexpected::Unsigned(v as u64),
                                                 &"variant index 0 <= i < 4"))
                    }
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where E: Error
            {
                match v {
                    "None" => Ok(Variant::None),
                    "Domain" => Ok(Variant::Domain),
                    "Ipv4" => Ok(Variant::Ipv4),
                    "Ipv6" => Ok(Variant::Ipv6),
                    _ => Err(Error::unknown_variant(v, VARIANTS)),
                }
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where E: Error
            {
                match v {
                    b"None" => Ok(Variant::None),
                    b"Domain" => Ok(Variant::Domain),
                    b"Ipv4" => Ok(Variant::Ipv4),
                    b"Ipv6" => Ok(Variant::Ipv6),
                    _ => {
                        let s = String::from_utf8_lossy(v);
                        Err(Error::unknown_variant(&s, VARIANTS))
                    }
                }
            }
        }

        impl<'de> Deserialize<'de> for Variant {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: Deserializer<'de>
            {
                deserializer.deserialize_identifier(VariantVisitor)
            }
        }

        struct HostInternalVisitor;

        impl<'de> Visitor<'de> for HostInternalVisitor {
            type Value = HostInternal;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("enum HostInternal")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                where A: EnumAccess<'de>
            {
                match try!(data.variant()) {
                    (Variant::None, variant) => {
                        variant.unit_variant().map(|()| HostInternal::None)
                    }
                    (Variant::Domain, variant) => {
                        variant.unit_variant().map(|()| HostInternal::Domain)
                    }
                    (Variant::Ipv4, variant) => {
                        variant.newtype_variant().map(HostInternal::Ipv4)
                    }
                    (Variant::Ipv6, variant) => {
                        variant.newtype_variant().map(HostInternal::Ipv6)
                    }
                }
            }
        }

        const VARIANTS: &'static [&'static str] = &["None", "Domain", "Ipv4", "Ipv6"];
        deserializer.deserialize_enum("HostInternal", VARIANTS, HostInternalVisitor)
    }
}

// This is equivalent to:
//
//    #[derive(Serialize)]
//    enum Host<S> {
//        Domain(S),
//        Ipv4(Ipv4Addr),
//        Ipv6(Ipv6Addr),
//    }
impl<S> Serialize for Host<S>
    where S: Serialize
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
        where Ser: Serializer
    {
        match *self {
            Host::Domain(ref s) => {
                serializer.serialize_newtype_variant("Host", 0, "Domain", s)
            }
            Host::Ipv4(ref addr) => {
                serializer.serialize_newtype_variant("Host", 1, "Ipv4", addr)
            }
            Host::Ipv6(ref addr) => {
                serializer.serialize_newtype_variant("Host", 2, "Ipv6", addr)
            }
        }
    }
}

// This is equivalent to:
//
//    #[derive(Deserialize)]
//    enum Host<S> {
//        Domain(S),
//        Ipv4(Ipv4Addr),
//        Ipv6(Ipv6Addr),
//    }
impl<'de, S> Deserialize<'de> for Host<S>
    where S: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        enum Variant {
            Domain,
            Ipv4,
            Ipv6,
        }

        struct VariantVisitor;

        // This is equivalent to:
        //
        //    #[derive(Deserialize)]
        //    #[serde(variant_identifier)]
        //    enum Variant {
        //        Domain,
        //        Ipv4,
        //        Ipv6,
        //    }
        impl<'de> Visitor<'de> for VariantVisitor {
            type Value = Variant;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("variant identifier")
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
                where E: Error
            {
                match v {
                    0 => Ok(Variant::Domain),
                    1 => Ok(Variant::Ipv4),
                    2 => Ok(Variant::Ipv6),
                    _ => {
                        Err(Error::invalid_value(Unexpected::Unsigned(v as u64),
                                                 &"variant index 0 <= i < 3"))
                    }
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where E: Error
            {
                match v {
                    "Domain" => Ok(Variant::Domain),
                    "Ipv4" => Ok(Variant::Ipv4),
                    "Ipv6" => Ok(Variant::Ipv6),
                    _ => Err(Error::unknown_variant(v, VARIANTS)),
                }
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                where E: Error
            {
                match v {
                    b"Domain" => Ok(Variant::Domain),
                    b"Ipv4" => Ok(Variant::Ipv4),
                    b"Ipv6" => Ok(Variant::Ipv6),
                    _ => {
                        let s = String::from_utf8_lossy(v);
                        Err(Error::unknown_variant(&s, VARIANTS))
                    }
                }
            }
        }

        impl<'de> Deserialize<'de> for Variant {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where D: Deserializer<'de>
            {
                deserializer.deserialize_identifier(VariantVisitor)
            }
        }

        struct HostVisitor<S> {
            domain: PhantomData<S>,
        }

        impl<'de, S> Visitor<'de> for HostVisitor<S>
            where S: Deserialize<'de>
        {
            type Value = Host<S>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("enum Host")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
                where A: EnumAccess<'de>
            {
                match try!(data.variant()) {
                    (Variant::Domain, variant) => {
                        variant.newtype_variant().map(Host::Domain)
                    }
                    (Variant::Ipv4, variant) => {
                        variant.newtype_variant().map(Host::Ipv4)
                    }
                    (Variant::Ipv6, variant) => {
                        variant.newtype_variant().map(Host::Ipv6)
                    }
                }
            }
        }

        const VARIANTS: &'static [&'static str] = &["Domain", "Ipv4", "Ipv6"];
        let visitor = HostVisitor { domain: PhantomData };
        deserializer.deserialize_enum("Host", VARIANTS, visitor)
    }
}

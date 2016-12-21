// WARNING: Do not modify this file if it is `host.rs`.
//
// Instead, modify `host.rs.in` and rerun the `serde-code-generator`.

#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
#[cfg(feature = "serde")]
const _IMPL_DESERIALIZE_FOR_HostInternal: () =
    {
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::de::Deserialize for HostInternal {
            fn deserialize<__D>(deserializer: &mut __D)
             -> ::std::result::Result<HostInternal, __D::Error> where
             __D: _serde::de::Deserializer {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                    __ignore,
                }
                impl _serde::de::Deserialize for __Field {
                    #[inline]
                    fn deserialize<__D>(deserializer: &mut __D)
                     -> ::std::result::Result<__Field, __D::Error> where
                     __D: _serde::de::Deserializer {
                        struct __FieldVisitor;
                        impl _serde::de::Visitor for __FieldVisitor {
                            type
                            Value
                            =
                            __Field;
                            fn visit_usize<__E>(&mut self, value: usize)
                             -> ::std::result::Result<__Field, __E> where
                             __E: _serde::de::Error {
                                match value {
                                    0usize => { Ok(__Field::__field0) }
                                    1usize => { Ok(__Field::__field1) }
                                    2usize => { Ok(__Field::__field2) }
                                    3usize => { Ok(__Field::__field3) }
                                    _ =>
                                    Err(_serde::de::Error::invalid_value("expected a variant")),
                                }
                            }
                            fn visit_str<__E>(&mut self, value: &str)
                             -> ::std::result::Result<__Field, __E> where
                             __E: _serde::de::Error {
                                match value {
                                    "None" => { Ok(__Field::__field0) }
                                    "Domain" => { Ok(__Field::__field1) }
                                    "Ipv4" => { Ok(__Field::__field2) }
                                    "Ipv6" => { Ok(__Field::__field3) }
                                    _ =>
                                    Err(_serde::de::Error::unknown_variant(value)),
                                }
                            }
                            fn visit_bytes<__E>(&mut self, value: &[u8])
                             -> ::std::result::Result<__Field, __E> where
                             __E: _serde::de::Error {
                                match value {
                                    b"None" => { Ok(__Field::__field0) }
                                    b"Domain" => { Ok(__Field::__field1) }
                                    b"Ipv4" => { Ok(__Field::__field2) }
                                    b"Ipv6" => { Ok(__Field::__field3) }
                                    _ => {
                                        let value =
                                            ::std::string::String::from_utf8_lossy(value);
                                        Err(_serde::de::Error::unknown_variant(&value))
                                    }
                                }
                            }
                        }
                        deserializer.deserialize_struct_field(__FieldVisitor)
                    }
                }
                struct __Visitor;
                impl _serde::de::EnumVisitor for __Visitor {
                    type
                    Value
                    =
                    HostInternal;
                    fn visit<__V>(&mut self, mut visitor: __V)
                     -> ::std::result::Result<HostInternal, __V::Error> where
                     __V: _serde::de::VariantVisitor {
                        match try!(visitor . visit_variant (  )) {
                            __Field::__field0 => {
                                try!(visitor . visit_unit (  ));
                                Ok(HostInternal::None)
                            }
                            __Field::__field1 => {
                                try!(visitor . visit_unit (  ));
                                Ok(HostInternal::Domain)
                            }
                            __Field::__field2 =>
                            Ok(HostInternal::Ipv4(try!(visitor . visit_newtype
                                                       :: < Ipv4Addr > (
                                                       )))),
                            __Field::__field3 =>
                            Ok(HostInternal::Ipv6(try!(visitor . visit_newtype
                                                       :: < Ipv6Addr > (
                                                       )))),
                            __Field::__ignore => {
                                Err(_serde::de::Error::end_of_stream())
                            }
                        }
                    }
                }
                const VARIANTS: &'static [&'static str] =
                    &["None", "Domain", "Ipv4", "Ipv6"];
                deserializer.deserialize_enum("HostInternal", VARIANTS,
                                              __Visitor)
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
#[cfg(feature = "serde")]
const _IMPL_SERIALIZE_FOR_HostInternal: () =
    {
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::ser::Serialize for HostInternal {
            fn serialize<__S>(&self, _serializer: &mut __S)
             -> ::std::result::Result<(), __S::Error> where
             __S: _serde::ser::Serializer {
                match *self {
                    HostInternal::None =>
                    _serde::ser::Serializer::serialize_unit_variant(_serializer,
                                                                    "HostInternal",
                                                                    0usize,
                                                                    "None"),
                    HostInternal::Domain =>
                    _serde::ser::Serializer::serialize_unit_variant(_serializer,
                                                                    "HostInternal",
                                                                    1usize,
                                                                    "Domain"),
                    HostInternal::Ipv4(ref __simple_value) =>
                    _serde::ser::Serializer::serialize_newtype_variant(_serializer,
                                                                       "HostInternal",
                                                                       2usize,
                                                                       "Ipv4",
                                                                       __simple_value),
                    HostInternal::Ipv6(ref __simple_value) =>
                    _serde::ser::Serializer::serialize_newtype_variant(_serializer,
                                                                       "HostInternal",
                                                                       3usize,
                                                                       "Ipv6",
                                                                       __simple_value),
                }
            }
        }
    };
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum HostInternal { None, Domain, Ipv4(Ipv4Addr), Ipv6(Ipv6Addr), }

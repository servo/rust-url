
// Components
// Before ':'
// Before ':' (if a password is given) or '@' (if not)
// Before initial '/', if any
// Before '?', unlike Position::QueryStart
// Before '#', unlike Position::FragmentStart
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_DESERIALIZE_FOR_Url: () =
    {
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::de::Deserialize for Url {
            fn deserialize<__D>(deserializer: &mut __D)
             -> ::std::result::Result<Url, __D::Error> where
             __D: _serde::de::Deserializer {
                #[allow(non_camel_case_types)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __field3,
                    __field4,
                    __field5,
                    __field6,
                    __field7,
                    __field8,
                    __field9,
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
                                    4usize => { Ok(__Field::__field4) }
                                    5usize => { Ok(__Field::__field5) }
                                    6usize => { Ok(__Field::__field6) }
                                    7usize => { Ok(__Field::__field7) }
                                    8usize => { Ok(__Field::__field8) }
                                    9usize => { Ok(__Field::__field9) }
                                    _ => Ok(__Field::__ignore),
                                }
                            }
                            fn visit_str<__E>(&mut self, value: &str)
                             -> ::std::result::Result<__Field, __E> where
                             __E: _serde::de::Error {
                                match value {
                                    "serialization" => {
                                        Ok(__Field::__field0)
                                    }
                                    "scheme_end" => { Ok(__Field::__field1) }
                                    "username_end" => {
                                        Ok(__Field::__field2)
                                    }
                                    "host_start" => { Ok(__Field::__field3) }
                                    "host_end" => { Ok(__Field::__field4) }
                                    "host" => { Ok(__Field::__field5) }
                                    "port" => { Ok(__Field::__field6) }
                                    "path_start" => { Ok(__Field::__field7) }
                                    "query_start" => { Ok(__Field::__field8) }
                                    "fragment_start" => {
                                        Ok(__Field::__field9)
                                    }
                                    _ => Ok(__Field::__ignore),
                                }
                            }
                            fn visit_bytes<__E>(&mut self, value: &[u8])
                             -> ::std::result::Result<__Field, __E> where
                             __E: _serde::de::Error {
                                match value {
                                    b"serialization" => {
                                        Ok(__Field::__field0)
                                    }
                                    b"scheme_end" => { Ok(__Field::__field1) }
                                    b"username_end" => {
                                        Ok(__Field::__field2)
                                    }
                                    b"host_start" => { Ok(__Field::__field3) }
                                    b"host_end" => { Ok(__Field::__field4) }
                                    b"host" => { Ok(__Field::__field5) }
                                    b"port" => { Ok(__Field::__field6) }
                                    b"path_start" => { Ok(__Field::__field7) }
                                    b"query_start" => {
                                        Ok(__Field::__field8)
                                    }
                                    b"fragment_start" => {
                                        Ok(__Field::__field9)
                                    }
                                    _ => Ok(__Field::__ignore),
                                }
                            }
                        }
                        deserializer.deserialize_struct_field(__FieldVisitor)
                    }
                }
                struct __Visitor;
                impl _serde::de::Visitor for __Visitor {
                    type
                    Value
                    =
                    Url;
                    #[inline]
                    fn visit_seq<__V>(&mut self, mut visitor: __V)
                     -> ::std::result::Result<Url, __V::Error> where
                     __V: _serde::de::SeqVisitor {
                        let __field0 =
                            match try!(visitor . visit :: < String > (  )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(0usize));
                                }
                            };
                        let __field1 =
                            match try!(visitor . visit :: < u32 > (  )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(1usize));
                                }
                            };
                        let __field2 =
                            match try!(visitor . visit :: < u32 > (  )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(2usize));
                                }
                            };
                        let __field3 =
                            match try!(visitor . visit :: < u32 > (  )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(3usize));
                                }
                            };
                        let __field4 =
                            match try!(visitor . visit :: < u32 > (  )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(4usize));
                                }
                            };
                        let __field5 =
                            match try!(visitor . visit :: < HostInternal > (
                                       )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(5usize));
                                }
                            };
                        let __field6 =
                            match try!(visitor . visit :: < Option < u16 > > (
                                        )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(6usize));
                                }
                            };
                        let __field7 =
                            match try!(visitor . visit :: < u32 > (  )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(7usize));
                                }
                            };
                        let __field8 =
                            match try!(visitor . visit :: < Option < u32 > > (
                                        )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(8usize));
                                }
                            };
                        let __field9 =
                            match try!(visitor . visit :: < Option < u32 > > (
                                        )) {
                                Some(value) => { value }
                                None => {
                                    try!(visitor . end (  ));
                                    return Err(_serde::de::Error::invalid_length(9usize));
                                }
                            };
                        try!(visitor . end (  ));
                        Ok(Url{serialization: __field0,
                               scheme_end: __field1,
                               username_end: __field2,
                               host_start: __field3,
                               host_end: __field4,
                               host: __field5,
                               port: __field6,
                               path_start: __field7,
                               query_start: __field8,
                               fragment_start: __field9,})
                    }
                    #[inline]
                    fn visit_map<__V>(&mut self, mut visitor: __V)
                     -> ::std::result::Result<Url, __V::Error> where
                     __V: _serde::de::MapVisitor {
                        let mut __field0: Option<String> = None;
                        let mut __field1: Option<u32> = None;
                        let mut __field2: Option<u32> = None;
                        let mut __field3: Option<u32> = None;
                        let mut __field4: Option<u32> = None;
                        let mut __field5: Option<HostInternal> = None;
                        let mut __field6: Option<Option<u16>> = None;
                        let mut __field7: Option<u32> = None;
                        let mut __field8: Option<Option<u32>> = None;
                        let mut __field9: Option<Option<u32>> = None;
                        while let Some(key) =
                                  try!(visitor . visit_key :: < __Field > (
                                       )) {
                            match key {
                                __Field::__field0 => {
                                    if __field0.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("serialization"));
                                    }
                                    __field0 =
                                        Some(try!(visitor . visit_value :: <
                                                  String > (  )));
                                }
                                __Field::__field1 => {
                                    if __field1.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("scheme_end"));
                                    }
                                    __field1 =
                                        Some(try!(visitor . visit_value :: <
                                                  u32 > (  )));
                                }
                                __Field::__field2 => {
                                    if __field2.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("username_end"));
                                    }
                                    __field2 =
                                        Some(try!(visitor . visit_value :: <
                                                  u32 > (  )));
                                }
                                __Field::__field3 => {
                                    if __field3.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("host_start"));
                                    }
                                    __field3 =
                                        Some(try!(visitor . visit_value :: <
                                                  u32 > (  )));
                                }
                                __Field::__field4 => {
                                    if __field4.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("host_end"));
                                    }
                                    __field4 =
                                        Some(try!(visitor . visit_value :: <
                                                  u32 > (  )));
                                }
                                __Field::__field5 => {
                                    if __field5.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("host"));
                                    }
                                    __field5 =
                                        Some(try!(visitor . visit_value :: <
                                                  HostInternal > (  )));
                                }
                                __Field::__field6 => {
                                    if __field6.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("port"));
                                    }
                                    __field6 =
                                        Some(try!(visitor . visit_value :: <
                                                  Option < u16 > > (  )));
                                }
                                __Field::__field7 => {
                                    if __field7.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("path_start"));
                                    }
                                    __field7 =
                                        Some(try!(visitor . visit_value :: <
                                                  u32 > (  )));
                                }
                                __Field::__field8 => {
                                    if __field8.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("query_start"));
                                    }
                                    __field8 =
                                        Some(try!(visitor . visit_value :: <
                                                  Option < u32 > > (  )));
                                }
                                __Field::__field9 => {
                                    if __field9.is_some() {
                                        return Err(<__V::Error as
                                                       _serde::de::Error>::duplicate_field("fragment_start"));
                                    }
                                    __field9 =
                                        Some(try!(visitor . visit_value :: <
                                                  Option < u32 > > (  )));
                                }
                                _ => {
                                    let _ =
                                        try!(visitor . visit_value :: < _serde
                                             :: de :: impls :: IgnoredAny > (
                                             ));
                                }
                            }
                        }
                        try!(visitor . end (  ));
                        let __field0 =
                            match __field0 {
                                Some(__field0) => __field0,
                                None =>
                                try!(visitor . missing_field ( "serialization"
                                     )),
                            };
                        let __field1 =
                            match __field1 {
                                Some(__field1) => __field1,
                                None =>
                                try!(visitor . missing_field ( "scheme_end"
                                     )),
                            };
                        let __field2 =
                            match __field2 {
                                Some(__field2) => __field2,
                                None =>
                                try!(visitor . missing_field ( "username_end"
                                     )),
                            };
                        let __field3 =
                            match __field3 {
                                Some(__field3) => __field3,
                                None =>
                                try!(visitor . missing_field ( "host_start"
                                     )),
                            };
                        let __field4 =
                            match __field4 {
                                Some(__field4) => __field4,
                                None =>
                                try!(visitor . missing_field ( "host_end" )),
                            };
                        let __field5 =
                            match __field5 {
                                Some(__field5) => __field5,
                                None =>
                                try!(visitor . missing_field ( "host" )),
                            };
                        let __field6 =
                            match __field6 {
                                Some(__field6) => __field6,
                                None =>
                                try!(visitor . missing_field ( "port" )),
                            };
                        let __field7 =
                            match __field7 {
                                Some(__field7) => __field7,
                                None =>
                                try!(visitor . missing_field ( "path_start"
                                     )),
                            };
                        let __field8 =
                            match __field8 {
                                Some(__field8) => __field8,
                                None =>
                                try!(visitor . missing_field ( "query_start"
                                     )),
                            };
                        let __field9 =
                            match __field9 {
                                Some(__field9) => __field9,
                                None =>
                                try!(visitor . missing_field (
                                     "fragment_start" )),
                            };
                        Ok(Url{serialization: __field0,
                               scheme_end: __field1,
                               username_end: __field2,
                               host_start: __field3,
                               host_end: __field4,
                               host: __field5,
                               port: __field6,
                               path_start: __field7,
                               query_start: __field8,
                               fragment_start: __field9,})
                    }
                }
                const FIELDS: &'static [&'static str] =
                    &["serialization", "scheme_end", "username_end",
                      "host_start", "host_end", "host", "port", "path_start",
                      "query_start", "fragment_start"];
                deserializer.deserialize_struct("Url", FIELDS, __Visitor)
            }
        }
    };
#[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
const _IMPL_SERIALIZE_FOR_Url: () =
    {
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::ser::Serialize for Url {
            fn serialize<__S>(&self, _serializer: &mut __S)
             -> ::std::result::Result<(), __S::Error> where
             __S: _serde::ser::Serializer {
                let mut __serde_state =
                    try!(_serializer . serialize_struct (
                         "Url" , 0 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "serialization" , & self .
                     serialization ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "scheme_end" , & self . scheme_end
                     ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "username_end" , & self .
                     username_end ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "host_start" , & self . host_start
                     ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "host_end" , & self . host_end ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "host" , & self . host ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "port" , & self . port ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "path_start" , & self . path_start
                     ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "query_start" , & self .
                     query_start ));
                try!(_serializer . serialize_struct_elt (
                     & mut __serde_state , "fragment_start" , & self .
                     fragment_start ));
                _serializer.serialize_struct_end(__serde_state)
            }
        }
    };
/// A parsed URL record.
#[derive(Clone)]
pub struct Url {
    /// Syntax in pseudo-BNF:
    ///
    ///   url = scheme ":" [ hierarchical | non-hierarchical ] [ "?" query ]? [ "#" fragment ]?
    ///   non-hierarchical = non-hierarchical-path
    ///   non-hierarchical-path = /* Does not start with "/" */
    ///   hierarchical = authority? hierarchical-path
    ///   authority = "//" userinfo? host [ ":" port ]?
    ///   userinfo = username [ ":" password ]? "@"
    ///   hierarchical-path = [ "/" path-segment ]+
    serialization: String,
    scheme_end: u32,
    username_end: u32,
    host_start: u32,
    host_end: u32,
    host: HostInternal,
    port: Option<u16>,
    path_start: u32,
    query_start: Option<u32>,
    fragment_start: Option<u32>,
}

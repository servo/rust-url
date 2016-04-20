// Copyright 2016 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use host::Host;
use idna::domain_to_unicode;
use parser::default_port;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use Url;

pub fn url_origin(url: &Url) -> Origin {
    let scheme = url.scheme();
    match scheme {
        "blob" => {
            let result = Url::parse(url.path());
            match result {
                Ok(ref url) => url_origin(url),
                Err(_)  => Origin::new_opaque()
            }
        },
        "ftp" | "gopher" | "http" | "https" | "ws" | "wss" => {
            Origin::Tuple(scheme.to_owned(), url.host().unwrap().to_owned(),
                url.port_or_known_default().unwrap())
        },
        // TODO: Figure out what to do if the scheme is a file
        "file" => Origin::new_opaque(),
        _ => Origin::new_opaque()
    }
}

/// The origin of an URL
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature="heap_size", derive(HeapSizeOf))]
pub enum Origin {
    /// A globally unique identifier
    Opaque(OpaqueOrigin),

    /// Consists of the URL's scheme, host and port
    Tuple(String, Host<String>, u16)
}


impl Origin {
    /// Creates a new opaque origin that is only equal to itself.
    pub fn new_opaque() -> Origin {
        static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        Origin::Opaque(OpaqueOrigin(COUNTER.fetch_add(1, Ordering::SeqCst)))
    }

    /// Return whether this origin is a (scheme, host, port) tuple
    /// (as opposed to an opaque origin).
    pub fn is_tuple(&self) -> bool {
        matches!(*self, Origin::Tuple(..))
    }

    /// https://html.spec.whatwg.org/multipage/#ascii-serialisation-of-an-origin
    pub fn ascii_serialization(&self) -> String {
        match *self {
            Origin::Opaque(_) => "null".to_owned(),
            Origin::Tuple(ref scheme, ref host, port) => {
                if default_port(scheme) == Some(port) {
                    format!("{}://{}", scheme, host)
                } else {
                    format!("{}://{}:{}", scheme, host, port)
                }
            }
        }
    }

    /// https://html.spec.whatwg.org/multipage/#unicode-serialisation-of-an-origin
    pub fn unicode_serialization(&self) -> String {
        match *self {
            Origin::Opaque(_) => "null".to_owned(),
            Origin::Tuple(ref scheme, ref host, port) => {
                let host = match *host {
                    Host::Domain(ref domain) => {
                        let (domain, _errors) = domain_to_unicode(domain);
                        Host::Domain(domain)
                    }
                    _ => host.clone()
                };
                if default_port(scheme) == Some(port) {
                    format!("{}://{}", scheme, host)
                } else {
                    format!("{}://{}:{}", scheme, host, port)
                }
            }
        }
    }
}

/// Opaque identifier for URLs that have file or other schemes
#[derive(Eq, PartialEq, Clone, Debug)]
#[cfg_attr(feature="heap_size", derive(HeapSizeOf))]
pub struct OpaqueOrigin(usize);

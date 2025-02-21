// Copyright 2016 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::host::Host;
use crate::Url;
use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Get the origin from a URL according to the specification:
/// <https://url.spec.whatwg.org/#origin>
pub fn url_origin(url: &Url) -> Origin {
    let scheme = url.scheme();
    match scheme {
        // > "blob"
        // > 1. If url’s blob URL entry is non-null, then return url’s blob URL entry’s
        // >    environment’s origin.
        // > 2. Let pathURL be the result of parsing the result of URL path serializing url.
        // > 3. If pathURL is failure, then return a new opaque origin.
        // > 4. If pathURL’s scheme is "http", "https", or "file", then return pathURL’s origin.
        // > 5. Return a new opaque origin.
        "blob" => {
            let result = Url::parse(url.path());
            match result {
                Ok(ref url) => url_origin(url),
                Err(_) => Origin::new_opaque(),
            }
        }
        // > "ftp" "http" "https" "ws" "wss": Return the tuple origin (url’s scheme, url’s host,
        // > url’s port, null).
        //
        "ftp" | "http" | "https" | "ws" | "wss" => Origin::Tuple(
            scheme.to_owned(),
            url.host().unwrap().to_owned(),
            url.port(),
        ),
        // > "file": Unfortunate as it is, this is left as an exercise to the reader. When in
        // > doubt, return a new opaque origin.
        //
        // TODO: Figure out what to do if the scheme is a file
        "file" => Origin::new_opaque(),
        // > Otherwise: Return a new opaque origin.
        _ => Origin::new_opaque(),
    }
}

/// The origin of an URL
///
/// Two URLs with the same origin are considered
/// to originate from the same entity and can therefore trust
/// each other.
///
/// The origin is determined based on the scheme as follows:
///
/// - If the scheme is "blob" the origin is the origin of the
///   URL contained in the path component. If parsing fails,
///   it is an opaque origin.
/// - If the scheme is "ftp", "http", "https", "ws", or "wss",
///   then the origin is a tuple of the scheme, host, and port.
/// - If the scheme is anything else, the origin is opaque, meaning
///   the URL does not have the same origin as any other URL.
///
/// For more information see <https://url.spec.whatwg.org/#origin>
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Origin {
    /// A globally unique identifier
    Opaque(OpaqueOrigin),

    /// Consists of the URL's scheme, host and port
    Tuple(String, Host<String>, Option<u16>),
}

impl Origin {
    /// Creates a new opaque origin that is only equal to itself.
    pub fn new_opaque() -> Origin {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Origin::Opaque(OpaqueOrigin(COUNTER.fetch_add(1, Ordering::SeqCst)))
    }

    /// Return whether this origin is a (scheme, host, port) tuple
    /// (as opposed to an opaque origin).
    pub fn is_tuple(&self) -> bool {
        matches!(*self, Origin::Tuple(..))
    }

    /// <https://html.spec.whatwg.org/multipage/#ascii-serialisation-of-an-origin>
    pub fn ascii_serialization(&self) -> String {
        match *self {
            Origin::Opaque(_) => "null".to_owned(),
            Origin::Tuple(ref scheme, ref host, Some(port)) => {
                format!("{}://{}:{}", scheme, host, port)
            }
            Origin::Tuple(ref scheme, ref host, _) => {
                format!("{}://{}", scheme, host)
            }
        }
    }

    /// <https://html.spec.whatwg.org/multipage/#unicode-serialisation-of-an-origin>
    pub fn unicode_serialization(&self) -> String {
        match *self {
            Origin::Opaque(_) => "null".to_owned(),
            Origin::Tuple(ref scheme, ref host, port) => {
                let host = match *host {
                    Host::Domain(ref domain) => {
                        let (domain, _errors) = idna::domain_to_unicode(domain);
                        Host::Domain(domain)
                    }
                    _ => host.clone(),
                };
                match port {
                    Some(port) => format!("{}://{}:{}", scheme, host, port),
                    None => format!("{}://{}", scheme, host),
                }
            }
        }
    }
}

/// Opaque identifier for URLs that have file or other schemes
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct OpaqueOrigin(usize);

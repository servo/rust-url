// Copyright 2016 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use Url;
use host::Host;

impl Url {
    /// Return the origin of this URL (https://url.spec.whatwg.org/#origin)
    pub fn origin(&self) -> Origin {
        let scheme = self.scheme();
        match scheme {
            "blob" => {
                let result = Url::parse(self.path());
                match result {
                    Ok(ref url) => url.origin(),
                    Err(_)  => Origin::new_opaque()
                }
            },
            "ftp" | "gopher" | "http" | "https" | "ws" | "wss" => {
                Origin::Tuple(scheme.to_owned(), self.host().unwrap().to_owned(),
                    self.port_or_known_default().unwrap())
            },
            // TODO: Figure out what to do if the scheme is a file
            "file" => Origin::new_opaque(),
            _ => Origin::new_opaque()
        }
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
        Origin::Opaque(OpaqueOrigin(Box::new(0)))
    }
}

/// Opaque identifier for URLs that have file or other schemes
#[derive(Eq, Clone, Debug)]
#[cfg_attr(feature="heap_size", derive(HeapSizeOf))]
// `u8` is a dummy non-zero-sized type to force the allocator to return a unique pointer.
// (It returns `std::heap::EMPTY` for zero-sized allocations.)
pub struct OpaqueOrigin(Arc<u8>);

/// Note that `opaque_origin.clone() != opaque_origin`.
impl PartialEq for OpaqueOrigin {
    fn eq(&self, other: &Self) -> bool {
        let a: *const u8 = &*self.0;
        let b: *const u8 = &*other.0;
        a == b
    }
}

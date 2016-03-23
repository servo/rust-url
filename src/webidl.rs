// Copyright 2016 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use {Url, ParseError};
use host::Host;
use idna::domain_to_unicode;
use parser::{Parser, SchemeType, default_port};

/// https://url.spec.whatwg.org/#api
pub struct WebIdl;

impl WebIdl {
    /// https://url.spec.whatwg.org/#dom-url-domaintoascii
    pub fn domain_to_ascii(domain: &str) -> String {
        match Host::parse(domain) {
            Ok(Host::Domain(domain)) => domain,
            _ => String::new(),
        }
    }

    /// https://url.spec.whatwg.org/#dom-url-domaintounicode
    pub fn domain_to_unicode(domain: &str) -> String {
        match Host::parse(domain) {
            Ok(Host::Domain(ref domain)) => {
                let (unicode, _errors) = domain_to_unicode(domain);
                unicode
            }
            _ => String::new(),
        }
    }

    pub fn href(url: &Url) -> &str {
        &url.serialization
    }

    pub fn set_href(url: &mut Url, value: &str) -> Result<(), ParseError> {
        *url = try!(Url::parse(value));
        Ok(())
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-origin
    pub fn origin(url: &Url) -> String {
        url.origin().unicode_serialization()
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-protocol
    #[inline]
    pub fn protocol(url: &Url) -> &str {
        debug_assert!(url.byte_at(url.scheme_end) == b':');
        url.slice(..url.scheme_end + 1)
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-protocol
    pub fn set_protocol(url: &mut Url, new_protocol: &str) {
        let _ = url.set_scheme_internal(new_protocol, true);
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-username
    #[inline]
    pub fn username(url: &Url) -> &str {
        url.username()
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-username
    pub fn set_username(url: &mut Url, new_username: &str) {
        let _ = url.set_username(new_username);
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-password
    #[inline]
    pub fn password(url: &Url) -> &str {
        url.password().unwrap_or("")
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-password
    pub fn set_password(url: &mut Url, new_password: &str) {
        let _ = url.set_password(if new_password.is_empty() { None } else { Some(new_password) });
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-host
    #[inline]
    pub fn host(url: &Url) -> &str {
        let host = url.slice(url.host_start..url.path_start);
        host
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-host
    pub fn set_host(url: &mut Url, new_host: &str) {
        if url.cannot_be_a_base() {
            return
        }
        let host;
        let opt_port;
        {
            let scheme = url.scheme();
            let result = Parser::parse_host(new_host, SchemeType::from(scheme), |_| ());
            match result {
                Ok((h, remaining)) => {
                    host = h;
                    opt_port = if remaining.starts_with(':') {
                        Parser::parse_port(remaining, |_| (), || default_port(scheme))
                        .ok().map(|(port, _remaining)| port)
                    } else {
                        None
                    };
                }
                Err(_) => return
            }
        }
        url.set_host_internal(host, opt_port)
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-hostname
    #[inline]
    pub fn hostname(url: &Url) -> &str {
        url.host_str().unwrap_or("")
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-hostname
    pub fn set_hostname(url: &mut Url, new_hostname: &str) {
        if url.cannot_be_a_base() {
            return
        }
        let result = Parser::parse_host(new_hostname, SchemeType::from(url.scheme()), |_| ());
        if let Ok((host, _remaining)) = result {
            url.set_host_internal(host, None)
        }
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-port
    #[inline]
    pub fn port(url: &Url) -> &str {
        if url.port.is_some() {
            debug_assert!(url.byte_at(url.host_end) == b':');
            url.slice(url.host_end + 1..url.path_start)
        } else {
            ""
        }
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-port
    pub fn set_port(url: &mut Url, new_port: &str) {
        let result;
        {
            // has_host implies !cannot_be_a_base
            let scheme = url.scheme();
            if !url.has_host() || scheme == "file" {
                return
            }
            result = Parser::parse_port(new_port, |_| (), || default_port(scheme))
        }
        if let Ok((new_port, _remaining)) = result {
            url.set_port_internal(new_port)
        }
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-pathname
    #[inline]
    pub fn pathname(url: &Url) -> &str {
         url.path()
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-pathname
    pub fn set_pathname(url: &mut Url, new_pathname: &str) {
        if !url.cannot_be_a_base() {
            url.set_path(new_pathname)
        }
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-search
    pub fn search(url: &Url) -> &str {
        match (url.query_start, url.fragment_start) {
            (Some(query_start), None) if {
                debug_assert!(url.byte_at(query_start) == b'?');
                // If the query (after ?) is not empty
                (query_start as usize) < url.serialization.len() - 1
            } => url.slice(query_start..),

            (Some(query_start), Some(fragment_start)) if {
                debug_assert!(url.byte_at(query_start) == b'?');
                // If the fragment (after ?) is not empty
                query_start < fragment_start
            } => url.slice(query_start..fragment_start),

            _ => "",
        }
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-search
    pub fn set_search(url: &mut Url, new_search: &str) {
        url.set_query(match new_search {
            "" => None,
            _ if new_search.starts_with('?') => Some(&new_search[1..]),
            _ => Some(new_search),
        })
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-searchparams
    pub fn search_params(url: &Url) -> Vec<(String, String)> {
        url.query_pairs().unwrap_or_else(Vec::new)
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-hash
    pub fn hash(url: &Url) -> &str {
        match url.fragment_start {
            Some(start) if {
                debug_assert!(url.byte_at(start) == b'#');
                // If the fragment (after #) is not empty
                (start as usize) < url.serialization.len() - 1
            } => url.slice(start..),
            _ => "",
        }
    }

    /// Setter for https://url.spec.whatwg.org/#dom-url-hash
    pub fn set_hash(url: &mut Url, new_hash: &str) {
        if url.scheme() != "javascript" {
            url.set_fragment(match new_hash {
                "" => None,
                _ if new_hash.starts_with('#') => Some(&new_hash[1..]),
                _ => Some(new_hash),
            })
        }
    }
}

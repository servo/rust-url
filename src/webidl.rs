// Copyright 2016 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use {Url, ParseError};

/// https://url.spec.whatwg.org/#api
pub struct WebIdl;

impl WebIdl {
    /// **Not implemented yet** https://url.spec.whatwg.org/#dom-url-domaintoascii
    pub fn domain_to_ascii(_domain: &str) -> String {
        unimplemented!()  // FIXME
    }

    /// **Not implemented yet** https://url.spec.whatwg.org/#dom-url-domaintounicode
    pub fn domain_to_unicode(_domain: &str) -> String {
        unimplemented!()  // FIXME
    }

    pub fn href(url: &Url) -> &str {
        &url.serialization
    }

    pub fn set_href(url: &mut Url, value: &str) -> Result<(), ParseError> {
        *url = try!(Url::parse(value));
        Ok(())
    }

    /// **Not implemented yet** Getter for https://url.spec.whatwg.org/#dom-url-origin
    pub fn get_origin(_url: &Url) -> String {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-protocol
    #[inline]
    pub fn get_protocol(url: &Url) -> &str {
        debug_assert!(url.byte_at(url.scheme_end) == b':');
        url.slice(..url.scheme_end + 1)
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-protocol
    pub fn set_protocol(_url: &mut Url, _new_protocol: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-username
    #[inline]
    pub fn get_username(url: &Url) -> &str {
        url.username()
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-username
    pub fn set_username(_url: &mut Url, _new_username: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-password
    #[inline]
    pub fn get_password(url: &Url) -> &str {
        url.password().unwrap_or("")
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-password
    pub fn set_password(_url: &mut Url, _new_password: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-host
    #[inline]
    pub fn get_host(url: &Url) -> &str {
        let host = url.slice(url.host_start..url.host_end);
        debug_assert!(!host.is_empty() || url.non_relative);
        host
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-host
    pub fn set_host(_url: &mut Url, _new_host: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-hostname
    #[inline]
    pub fn get_hostname(url: &Url) -> &str {
        url.host_str().unwrap_or("")
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-hostname
    pub fn set_hostname(_url: &mut Url, _new_hostname: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-port
    #[inline]
    pub fn get_port(url: &Url) -> &str {
        if url.port.is_some() {
            debug_assert!(url.byte_at(url.host_end) == b':');
            url.slice(url.host_end + 1..url.path_start)
        } else {
            ""
        }
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-port
    pub fn set_port(_url: &mut Url, _new_port: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-pathname
    #[inline]
    pub fn get_pathname(url: &Url) -> &str {
         url.path()
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-pathname
    pub fn set_pathname(_url: &mut Url, _new_pathname: &str) {
        unimplemented!()  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-search
    pub fn get_search(url: &Url) -> &str {
        match (url.query_start, url.fragment_start) {
            (None, _) => "",
            (Some(query_start), None) => url.slice(query_start..),
            (Some(query_start), Some(fragment_start)) => {
                url.slice(query_start..fragment_start)
            }
        }
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-search
    pub fn set_search(_url: &mut Url, _new_search: &str) {
        unimplemented!()  // FIXME
    }

    /// **Not implemented yet** Getter for https://url.spec.whatwg.org/#dom-url-searchparams
    pub fn get_search_params(_url: &Url) -> Vec<(String, String)> {
        unimplemented!();  // FIXME
    }

    /// Getter for https://url.spec.whatwg.org/#dom-url-hash
    pub fn get_hash(url: &Url) -> &str {
        match url.fragment_start {
            Some(start) => url.slice(start..),
            None => "",
        }
    }

    /// **Not implemented yet** Setter for https://url.spec.whatwg.org/#dom-url-hash
    pub fn set_hash(_url: &mut Url, _new_hash: &str) {
        unimplemented!()  // FIXME
    }
}

// Copyright 2016 Simon Sapin.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::{Range, RangeFrom, RangeTo, RangeFull, Index};
use Url;

impl Index<RangeFull> for Url {
    type Output = str;
    fn index(&self, _: RangeFull) -> &str {
        &self.serialization
    }
}

impl Index<RangeFrom<Position>> for Url {
    type Output = str;
    fn index(&self, range: RangeFrom<Position>) -> &str {
        &self.serialization[self.index(range.start)..]
    }
}

impl Index<RangeTo<Position>> for Url {
    type Output = str;
    fn index(&self, range: RangeTo<Position>) -> &str {
        &self.serialization[..self.index(range.end)]
    }
}

impl Index<Range<Position>> for Url {
    type Output = str;
    fn index(&self, range: Range<Position>) -> &str {
        &self.serialization[self.index(range.start)..self.index(range.end)]
    }
}

/// Indicates a position within a URL based on its components.
///
/// A range of positions can be used for slicing `Url`:
///
/// ```rust
/// # use url::{Url, Position};
/// # fn something(some_url: Url) {
/// let serialization: &str = &some_url[..];
/// let serialization_without_fragment: &str = &some_url[..Position::QueryEnd];
/// let authority: &str = &some_url[Position::UsernameStart..Position::PortEnd];
/// let data_url_payload: &str = &some_url[Position::PathStart..Position::QueryEnd];
/// let scheme_relative: &str = &some_url[Position::UsernameStart..];
/// # }
/// ```
///
/// In a pseudo-grammar (where `[`…`]?` makes a sub-sequence optional),
/// URL components and delimiters that separate them are:
///
/// ```notrust
/// url =
///     scheme ":"
///     [ "//" [ username [ ":" password ]? "@" ]? host [ ":" port ]? ]
///     path [ "?" query ]? [ "#" fragment ]?
/// ```
///
/// When a given component is not present,
/// its "start" and "end" position are the same
/// (so that `&some_url[FooStart..FooEnd]` is the empty string)
/// and component ordering is preserved
/// (so that a missing query "is between" a path and a fragment).
///
/// The end of a component and the start of the next are either the same or separate
/// by a delimiter.
/// (Not that the initial `/` of a path is considered part of the path here, not a delimiter.)
/// For example, `&url[..FragmentStart]` would include a `#` delimiter (if present in `url`),
/// so `&url[..QueryEnd]` might be desired instead.
///
/// `SchemeStart` and `FragmentEnd` are always the start and end of the entire URL,
/// so `&url[SchemeStart..X]` is the same as `&url[..X]`
/// and `&url[X..FragmentEnd]` is the same as `&url[X..]`.
pub enum Position {
    SchemeStart,
    SchemeEnd,
    UsernameStart,
    UsernameEnd,
    PasswordStart,
    PasswordEnd,
    HostStart,
    HostEnd,
    PortStart,
    PortEnd,
    PathStart,
    PathEnd,
    QueryStart,
    QueryEnd,
    FragmentStart,
    FragmentEnd
}

impl Url {
    #[inline]
    fn index(&self, position: Position) -> usize {
        match position {
            Position::SchemeStart => 0,

            Position::SchemeEnd => self.scheme_end as usize,

            Position::UsernameStart => if self.non_relative {
                debug_assert!(self.byte_at(self.scheme_end) == b':');
                debug_assert!(self.scheme_end + ":".len() as u32 == self.username_end);
                self.scheme_end as usize + ":".len()
            } else {
                debug_assert!(self.slice(self.scheme_end..).starts_with("://"));
                self.scheme_end as usize + "://".len()
            },

            Position::UsernameEnd => self.username_end as usize,

            Position::PasswordStart => if self.port.is_some() {
                debug_assert!(self.has_host());
                debug_assert!(self.byte_at(self.username_end) == b':');
                self.username_end as usize + ":".len()
            } else {
                debug_assert!(self.username_end == self.host_start);
                self.username_end as usize
            },

            Position::PasswordEnd => if self.port.is_some() {
                debug_assert!(self.has_host());
                debug_assert!(self.byte_at(self.username_end) == b':');
                debug_assert!(self.byte_at(self.host_start - "@".len() as u32) == b'@');
                self.host_start as usize - "@".len()
            } else {
                debug_assert!(self.username_end == self.host_start);
                self.host_start as usize
            },

            Position::HostStart => self.host_start as usize,

            Position::HostEnd => self.host_end as usize,

            Position::PortStart => if self.port.is_some() {
                debug_assert!(self.byte_at(self.host_end) == b':');
                self.host_end as usize + ":".len()
            } else {
                self.host_end as usize
            },

            Position::PortEnd => self.path_start as usize,

            Position::PathStart => self.path_start as usize,

            Position::PathEnd => match (self.query_start, self.fragment_start) {
                (Some(q), _) => q as usize,
                (None, Some(f)) => f as usize,
                (None, None) => self.serialization.len(),
            },

            Position::QueryStart => match (self.query_start, self.fragment_start) {
                (Some(q), _) => {
                    debug_assert!(self.byte_at(q) == b'?');
                    q as usize + "?".len()
                }
                (None, Some(f)) => f as usize,
                (None, None) => self.serialization.len(),
            },

            Position::QueryEnd => match self.fragment_start {
                None => self.serialization.len(),
                Some(f) => f as usize,
            },

            Position::FragmentStart => match self.fragment_start {
                Some(f) => {
                    debug_assert!(self.byte_at(f) == b'#');
                    f as usize + "#".len()
                }
                None => self.serialization.len(),
            },

            Position::FragmentEnd => self.serialization.len(),
        }
    }
}


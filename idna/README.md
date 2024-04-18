# `idna`

IDNA library for Rust implementing [UTS 46: Unicode IDNA Compatibility Processing](https://www.unicode.org/reports/tr46/) as parametrized by the [WHATWG URL Standard](https://url.spec.whatwg.org/#idna).

## What it does

* An implementation of UTS 46 is provided, with configurable ASCII deny list (e.g. STD3 or WHATWG rules).
* A callback mechanism is provided for pluggable logic for deciding if a label is deemed potentially too misleading to render as Unicode in a user interface.
* Errors are marked as U+FFFD REPLACEMENT CHARACTERs in Unicode output so that locations of errors may be illustrated to the user.

## What it does not do

* There is no default/sample policy provided for the callback mechanism mentioned above.
* Earlier variants of IDNA (2003, 2008) are not implemented—only UTS 46.
* There is no API for categorizing errors beyond there being an error.
* Checks that are configurable in UTS 46 but that the WHATWG URL Standard always set a particular way (regardless of the _beStrict_ flag in the URL Standard) cannot be configured (with the exception of the old deprecated API supporting transitional processing).

## Known spec violations

* The `verify_dns_length` behavior that this crate implements allows a trailing dot in the input as required by the UTS 46 test suite despite the UTS 46 spec saying that this isn't allowed.

## Breaking changes since 0.5.0

* IDNA 2008 rules are no longer supported. Attempting to enable them panics immediately.
* `check_hyphens` now also rejects the hyphen in the third and fourth position in a label.
* `domain_to_ascii_strict` now performs the _CheckHyphens_ check (matching previous documentation).
* The `Idna::to_ascii_inner` method has been removed. It didn't make sense as a public method, since callers were unable to figure out if there were errors. (A GitHub search found no callers for this method.)
* Punycode labels whose decoding does not yield any non-ASCII characters are now treated as being in error.

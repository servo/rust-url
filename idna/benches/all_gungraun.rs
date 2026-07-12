//! Callgrind-based instruction-count benchmarks for idna, mirroring the cases
//! in `all.rs`. Run with `cargo bench -p idna --bench all_gungraun` (requires
//! Valgrind and a matching `gungraun-runner`).

#![allow(deprecated)]

use std::borrow::Cow;
use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};

use idna::{AsciiDenyList, Config, Errors};

#[library_benchmark]
#[bench::puny_label("abc.xn--mgbcm")]
#[bench::ascii("example.com")]
#[bench::merged_label("Beispiel.xn--vermgensberater-ctb")]
fn to_unicode(encoded: &str) -> (String, Result<(), Errors>) {
    Config::default().to_unicode(black_box(encoded))
}

#[library_benchmark]
#[bench::already_puny_label("abc.xn--mgbcm")]
#[bench::puny_label("abc.ابج")]
#[bench::simple("example.com")]
#[bench::merged("beispiel.vermögensberater")]
fn to_ascii(encoded: &str) -> Result<String, Errors> {
    Config::default().to_ascii(black_box(encoded))
}

#[library_benchmark]
#[bench::plain("example.com".as_bytes())]
#[bench::hyphen("hyphenated-example.com".as_bytes())]
#[bench::leading_digit("1test.example".as_bytes())]
#[bench::unicode_mixed("مثال.example".as_bytes())]
#[bench::punycode_mixed("xn--mgbh0fb.example".as_bytes())]
#[bench::unicode_ltr("නම.උදාහරණ".as_bytes())]
#[bench::punycode_ltr("xn--r0co.xn--ozc8dl2c3bxd".as_bytes())]
#[bench::unicode_rtl("الاسم.مثال".as_bytes())]
#[bench::punycode_rtl("xn--mgba0b1dh.xn--mgbh0fb".as_bytes())]
fn to_ascii_cow(encoded: &[u8]) -> Result<Cow<'_, str>, Errors> {
    idna::domain_to_ascii_cow(black_box(encoded), AsciiDenyList::URL)
}

library_benchmark_group!(
    name = benches;
    benchmarks = to_unicode, to_ascii, to_ascii_cow
);

main!(library_benchmark_groups = benches);

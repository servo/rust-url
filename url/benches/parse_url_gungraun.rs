//! Callgrind-based instruction-count benchmarks for URL parsing, mirroring the
//! cases in `parse_url.rs`. Run with `cargo bench -p url --bench
//! parse_url_gungraun` (requires Valgrind and a matching `gungraun-runner`).

use std::hint::black_box;
use std::path::PathBuf;

use gungraun::{library_benchmark, library_benchmark_group, main};

use url::Url;

#[library_benchmark]
#[bench::short("https://example.com/bench")]
#[bench::long("https://example.com/parkbench?tre=es&st=uff")]
#[bench::fragment("https://example.com/parkbench?tre=es&st=uff#fragment")]
#[bench::plain("https://example.com/")]
#[bench::port("https://example.com:8080")]
#[bench::hyphen("https://hyphenated-example.com/")]
#[bench::leading_digit("https://1test.example/")]
#[bench::unicode_mixed("https://مثال.example/")]
#[bench::punycode_mixed("https://xn--mgbh0fb.example/")]
#[bench::unicode_ltr("https://නම.උදාහරණ/")]
#[bench::punycode_ltr("https://xn--r0co.xn--ozc8dl2c3bxd/")]
#[bench::unicode_rtl("https://الاسم.مثال/")]
#[bench::punycode_rtl("https://xn--mgba0b1dh.xn--mgbh0fb/")]
fn parse(url: &str) -> Url {
    black_box(url).parse::<Url>().unwrap()
}

fn setup_file_url() -> Url {
    let url = if cfg!(windows) {
        "file:///C:/dir/next_dir/sub_sub_dir/testing/testing.json"
    } else {
        "file:///data/dir/next_dir/sub_sub_dir/testing/testing.json"
    };
    url.parse::<Url>().unwrap()
}

#[library_benchmark]
#[bench::url_to_file_path(setup = setup_file_url)]
fn url_to_file_path(url: Url) -> PathBuf {
    black_box(url).to_file_path().unwrap()
}

library_benchmark_group!(
    name = benches;
    benchmarks = parse, url_to_file_path
);

main!(library_benchmark_groups = benches);

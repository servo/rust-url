#[macro_use]
extern crate bencher;

use bencher::{black_box, Bencher};

use url::Url;

fn short(bench: &mut Bencher) {
    let url = "https://example.com/bench";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn long(bench: &mut Bencher) {
    let url = "https://example.com/parkbench?tre=es&st=uff";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn plain(bench: &mut Bencher) {
    let url = "https://example.com/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn hyphen(bench: &mut Bencher) {
    let url = "https://hyphenated-example.com/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn leading_digit(bench: &mut Bencher) {
    let url = "https://1test.example/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn unicode_mixed(bench: &mut Bencher) {
    let url = "https://مثال.example/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn punycode_mixed(bench: &mut Bencher) {
    let url = "https://xn--mgbh0fb.example/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn unicode_ltr(bench: &mut Bencher) {
    let url = "https://නම.උදාහරණ/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn punycode_ltr(bench: &mut Bencher) {
    let url = "https://xn--r0co.xn--ozc8dl2c3bxd/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn unicode_rtl(bench: &mut Bencher) {
    let url = "https://الاسم.مثال/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

fn punycode_rtl(bench: &mut Bencher) {
    let url = "https://xn--mgba0b1dh.xn--mgbh0fb/";

    bench.bytes = url.len() as u64;
    bench.iter(|| black_box(url).parse::<Url>().unwrap());
}

benchmark_group!(
    benches,
    short,
    long,
    plain,
    hyphen,
    leading_digit,
    unicode_mixed,
    punycode_mixed,
    unicode_ltr,
    punycode_ltr,
    unicode_rtl,
    punycode_rtl,
);
benchmark_main!(benches);

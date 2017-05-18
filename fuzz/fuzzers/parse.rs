#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate url;
use std::str;

fuzz_target!(|data: &[u8]| {
	if let Ok(utf8) = str::from_utf8(data) {
		let url = url::Url::parse(utf8);
	}
});

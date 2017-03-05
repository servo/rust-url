#![no_main]

extern crate libfuzzer_sys;

extern crate url;
use std::slice;
use std::str;

#[export_name="LLVMFuzzerTestOneInput"]
pub extern fn go(data: *const u8, size: isize) -> i32 {
	let slice = unsafe { slice::from_raw_parts(data, size as usize) };
	if let Ok(utf8) = str::from_utf8(slice) {
		let url = url::Url::parse(utf8);
	}
	return 0;
}

use std::process::{Command, Stdio};
use std::io::Write;

fn main() {
    let mut child = Command::new(option_env!("RUSTC").unwrap_or("rustc"))
        .args(&["-", "--crate-type", "lib", "-Z", "no-trans"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"use std::net::IpAddr;").unwrap();
    if child.wait().unwrap().success() {
        // We can use `IpAddr` as it is `#[stable]` in this version of Rust.
        println!("cargo:rustc-cfg=has_ipaddr")
    }
}

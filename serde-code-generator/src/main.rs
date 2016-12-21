extern crate serde_codegen;

use std::path::Path;

pub fn main() {
    let src = Path::new("../src/codegen/url.rs.in");
    let dst = Path::new("../src/codegen/url.rs");

    serde_codegen::expand(&src, &dst).unwrap();

    let src = Path::new("../src/codegen/host.rs.in");
    let dst = Path::new("../src/codegen/host.rs");

    serde_codegen::expand(&src, &dst).unwrap();
}

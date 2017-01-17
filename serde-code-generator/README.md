# Serde Code Generator

It's complicated to write full featured serde implementations, but we don't
want to force our dependenices to depend on
[serde_codegen](https://serde.rs/codegen-stable.html), which takes a while to
compile. This executable avoids that overhead by externally regenerating these
implementations, so only the rust-url developers need to pay the cost of
compiling `serde_codegen`. It does come at the price of having to remember to
re-run the code generator if the types ever change.

To run, just execute `cargo run` and it will update the files in `src/codegen`.

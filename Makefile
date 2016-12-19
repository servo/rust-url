test:
	cargo test --features "query_encoding serde rustc-serialize"
	[ x$$TRAVIS_RUST_VERSION != xnightly ] || cargo test --features heapsize
	(cd idna && cargo test)

.PHONY: test

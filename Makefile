.PHONY: doc test-doc

doc:
	rustdoc src/url.rs -L target/deps -L target

test-doc:
	rustdoc src/url.rs -L target/deps -L target --test

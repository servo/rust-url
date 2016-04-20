test:
	cargo test --features "query_encoding serde rustc-serialize"
	[ x$$TRAVIS_RUST_VERSION != xnightly ] || cargo test --features heap_size

doc:
	cargo doc --features "query_encoding serde rustc-serialize"
	@echo '<meta http-equiv=refresh content=0;url=url/index.html>' > target/doc/index.html
	@cp github.png target/doc/

upload-doc: doc
	test "$(TRAVIS_BRANCH)" = master
	test "$(TRAVIS_PULL_REQUEST)" = false
	sudo pip install ghp-import
	ghp-import -n target/doc
	@git push -qf https://$(TOKEN)@github.com/$(TRAVIS_REPO_SLUG).git gh-pages

.PHONY: test doc upload-doc

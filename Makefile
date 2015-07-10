test:
	cargo test --features query_encoding
	cargo test --features serde_serialization
	cargo test

doc:
	cargo doc --features "query_encoding serde_serialization"
	@echo '<meta http-equiv=refresh content=0;url=url/index.html>' > target/doc/index.html
	@cp github.png target/doc/

upload-doc: doc
	test "$(TRAVIS_BRANCH)" = master
	test "$(TRAVIS_PULL_REQUEST)" = false
	sudo pip install ghp-import
	ghp-import -n target/doc
	git push -qf https://$(TOKEN)@github.com/$(TRAVIS_REPO_SLUG).git gh-pages

.PHONY: test doc upload-doc

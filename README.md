rust-url
========

Rust implementation of the [URL Standard](http://url.spec.whatwg.org/).

See [Rust bug #10707](https://github.com/mozilla/rust/issues/10707).

This depends on [rust-encoding](https://github.com/lifthrasiir/rust-encoding).


To do
-----

Not necessarily in the given order:

* Bikeshed the API
* Add `data:` URL parsing.
* Port rust-http and Servo.
* Add [IDNA support](http://url.spec.whatwg.org/#idna).
  Non-ASCII domains are a parse error for now.
  [Punycode](http://tools.ietf.org/html/rfc3492) is done,
  [Nameprep](http://tools.ietf.org/html/rfc3491) is the other big part.
* Add lots of tests.
  Contribute them to [web-platform-tests](https://github.com/w3c/web-platform-tests/tree/master/url).
* Report know/suspected spec bugs and test suite bugs.
* Refactor to reduce code duplication.
* Consider switching the spec from a state machine to functional style, like this code.

rust-url
========

[![Travis build Status](https://travis-ci.org/servo/rust-url.svg?branch=master)](https://travis-ci.org/servo/rust-url) [![Appveyor build status](https://ci.appveyor.com/api/projects/status/ulkqx2xcemyod6xa?svg=true)](https://ci.appveyor.com/project/Manishearth/rust-url)

Rust implementation of the [URL Standard](http://url.spec.whatwg.org/).

[Documentation](http://servo.github.io/rust-url/url/index.html)


To do
-----

Not necessarily in the given order:

* Deprecate and later remove rustc’s old liburl
* Add `data:` URL parsing.
* Add [IDNA support](http://url.spec.whatwg.org/#idna).
  Non-ASCII domains are a parse error for now.
  [Punycode](http://tools.ietf.org/html/rfc3492) is done,
  [Nameprep](http://tools.ietf.org/html/rfc3491) is the other big part.
* Add lots of tests.
  Contribute them to [web-platform-tests](https://github.com/w3c/web-platform-tests/tree/master/url).
* Consider switching the spec from a state machine to functional style, like this code.

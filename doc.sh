#!/bin/sh
rustdoc src/url.rs -L target/deps -L target "$@"

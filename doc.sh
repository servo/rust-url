#!/bin/sh
LIB=$(echo target/liburl*)
rustdoc src/lib.rs -L target/deps -L target --extern url=$LIB "$@"

#!/bin/sh
LIB=(target/liburl*)
rustdoc src/lib.rs -L target/deps -L target --extern url=$LIB "$@"

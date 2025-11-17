#!/bin/bash

# This script automates the process of publishing a package to crates.io.

# Publish each crate in the correct order
cargo publish -p cachelito-macro-utils
cargo publish -p cachelito-core
cargo publish -p cachelito-macros
cargo publish -p cachelito-async-macros
cargo publish -p cachelito-async
cargo publish -p cachelito
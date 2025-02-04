To run the Rust part:

```shell
cd rust
cargo build
cargo test

```

This will also build a bunch of `bazel-*` build files. 

You can also directly build the C++ stuff with `bazel build //...`. To clear the Bazel files, run `bazel clean --expunge`. 
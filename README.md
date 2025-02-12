# dpf-rs [![CI](https://github.com/0xWOLAND/dpf-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/0xWOLAND/dpf-rs/actions/workflows/ci.yml)

A Bazel-based implementation of Private Information Retrieval (PIR) client and server.

> [!WARNING]
> There are currently known build issues on macOS. We haven't found a reliable fix for these issues yet. If you discover a solution, please feel free to contribute it.

## Building

To build all targets:
```shell
bazel build //...
```

To run all tests:
```shell
bazel test //...
```
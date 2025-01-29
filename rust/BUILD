load("@rules_rust//proto/protobuf:toolchain.bzl", "PROTO_COMPILE_DEPS")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_doc", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "dpf",
    srcs = [
        "src/lib.rs",
        "src/client.rs",
        "src/server.rs",
    ],
    deps = [
        "//c:dpf_client",
        "//c:dpf_server",
        "@crates//:libc",
        "@crates//:protobuf",
    ] + PROTO_COMPILE_DEPS,
)
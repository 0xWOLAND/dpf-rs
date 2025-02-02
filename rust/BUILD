load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "dpf_pir",
    srcs = [
        "lib.rs",
        "client.rs",
        "database.rs",
        "server.rs"
    ],
    deps = [
        "//c:dpf_client",
        "//c:dpf_database",
        "//c:dpf_server",
        "@crates//:libc",
    ],
)

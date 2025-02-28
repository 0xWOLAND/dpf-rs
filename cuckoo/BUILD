load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

package(
    default_visibility = ["//visibility:public"],
)

rust_library(
    name = "cuckoo",
    srcs = glob(["src/lib.rs"]),
    deps = [
        "@crates//:rand",
        "@crates//:thiserror",
        "@crates//:hmac",
        "@crates//:sha2",
    ],
    edition = "2021",
)

rust_test(
    name = "unit_tests",
    crate = ":cuckoo",
    edition = "2021",
)
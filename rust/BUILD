load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "dpf-rs",
    srcs = [
        "src/lib.rs",
        "src/client.rs",
        "src/server.rs",
        "src/error.rs",
        "src/types.rs",
    ],
    edition = "2021",
    deps = [
        "//c:dpf_server",
        "//c:dpf_client",
        "//c:base64_utils",
        "//cuckoo:cuckoo",
        "@crates//:libc",
        "@crates//:thiserror",
        "@crates//:serde",
        "@crates//:serde_json",
        "@crates//:base64",
        "@crates//:rand",
    ],
)

rust_test(
    name = "dpf_unit_tests",
    crate = ":dpf-rs",
    edition = "2021",
)

rust_test(
    name = "dpf_integration_test",
    srcs = ["tests/integration_test.rs"],
    edition = "2021",
    deps = [":dpf-rs"],
)
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "talek",
    srcs = [
        "src/lib.rs",
        "src/client.rs",
        "src/server.rs",
        "src/error.rs",
        "src/constants.rs",
        "src/utils.rs",
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
        "@crates//:aes-gcm",
        "@crates//:ring",
    ],
)

rust_test(
    name = "talek_unit_tests",
    crate = ":talek",
    edition = "2021",
)

rust_test(
    name = "talek_integration_test",
    srcs = ["tests/integration_test.rs"],
    edition = "2021",
    deps = [
        ":talek",
        "//cuckoo:cuckoo",
        "@crates//:rand",
        "@crates//:base64",
    ],
)
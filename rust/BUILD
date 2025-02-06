load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_binary")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "dpf-rs",
    srcs = [
        "src/lib.rs",
        "src/client.rs",
        "src/server.rs",
    ],
    edition = "2021",  
    deps = [
        "//c:dpf_server", 
        "//c:dpf_client", 
        "@crates//:libc",
        "@crates//:thiserror",
    ],
)

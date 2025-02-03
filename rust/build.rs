use std::process::Command;
use std::path::PathBuf;
use std::env;

fn main() {
    let project_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();

    // Your existing rerun-if-changed declarations
    println!("cargo:rerun-if-changed=../c/server.cc");
    println!("cargo:rerun-if-changed=../c/server.h");
    println!("cargo:rerun-if-changed=../c/client.cc");
    println!("cargo:rerun-if-changed=../c/client.h");
    println!("cargo:rerun-if-changed=../c/base64_utils.cc");
    println!("cargo:rerun-if-changed=../c/base64_utils.h");
    println!("cargo:rerun-if-changed=../c/BUILD");

    // Build with Bazel
    let status = Command::new("bazel")
        .current_dir(&project_root)  
        .args(&["build", "//c:dpf_server", "//c:dpf_client", "//c:base64_utils"])
        .status()
        .expect("Failed to build C++ libraries with Bazel");
    
    if !status.success() {
        panic!("Bazel build failed");
    }

    // Add all necessary search paths
    let bazel_bin = project_root.join("bazel-bin").join("c");
    let external_path = project_root.join("bazel-bin").join("external");
    
    println!("cargo:rustc-link-search=native={}", bazel_bin.display());
    println!("cargo:rustc-link-search=native={}", external_path.join("boringssl").display());
    println!("cargo:rustc-link-search=native={}", external_path.join("com_google_absl").display());
    println!("cargo:rustc-link-search=native={}", external_path.join("com_github_protocolbuffers_protobuf").display());
    println!("cargo:rustc-link-search=native={}", external_path.join("google_dpf").display());
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");

    // Your core libraries
    println!("cargo:rustc-link-lib=static=dpf_server");
    println!("cargo:rustc-link-lib=static=dpf_client");
    println!("cargo:rustc-link-lib=static=base64_utils");

    // BoringSSL dependencies
    println!("cargo:rustc-link-lib=static=crypto");
    println!("cargo:rustc-link-lib=static=ssl");

    // Protobuf dependencies
    // println!("cargo:rustc-link-lib=static=protobuf");

    // Abseil dependencies
    // println!("cargo:rustc-link-lib=static=absl_status");
    // println!("cargo:rustc-link-lib=static=absl_statusor");
    // println!("cargo:rustc-link-lib=static=absl_base");
    // println!("cargo:rustc-link-lib=static=absl_bad_variant_access");
    // println!("cargo:rustc-link-lib=static=absl_bad_optional_access");

    // C++ standard library
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
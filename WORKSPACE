load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

http_archive(
    name = "google_dpf",
    urls = ["https://github.com/google/distributed_point_functions/archive/b8a36097c6954333fcd314c9b59d435b56472f27.zip"],
    strip_prefix = "distributed_point_functions-b8a36097c6954333fcd314c9b59d435b56472f27",
    sha256 = "c43fc82a2dc2d82cb38c7ac601118359c321f2f67c6c83416012b74af62bcc6f",
)

# rules_proto defines abstract rules for building Protocol Buffers.
# https://github.com/bazelbuild/rules_proto
http_archive(
    name = "rules_proto",
    sha256 = "5bc47cecbc756b40f88ec13029864ffe582bcb2688c17587e4a23a0bd009a268",
    strip_prefix = "rules_proto-71c4fc69900946093ac5c82d81efd19fa522d060",
    urls = [
        "https://github.com/bazelbuild/rules_proto/archive/71c4fc69900946093ac5c82d81efd19fa522d060.zip",
    ],
)

load("@rules_proto//proto:repositories.bzl", "rules_proto_dependencies", "rules_proto_toolchains")

rules_proto_dependencies()

rules_proto_toolchains()


# io_bazel_rules_go defines rules for generating C++ code from Protocol Buffers.
# https://github.com/bazelbuild/rules_go
http_archive(
    name = "io_bazel_rules_go",
    sha256 = "b492232e93aab8e0b57ae143ec1021369ff5b4a9763a04a1917e3fd79e178907",
    strip_prefix = "rules_go-7ed6bdea61e682b57adf4684ec1f47d174e817e4",
    urls = [
        "https://github.com/bazelbuild/rules_go/archive/7ed6bdea61e682b57adf4684ec1f47d174e817e4.zip",
    ],
)

load("@io_bazel_rules_go//go:deps.bzl", "go_register_toolchains", "go_rules_dependencies")

go_rules_dependencies()

go_register_toolchains(version = "1.19.3")

# Install gtest.
# https://github.com/google/googletest
http_archive(
    name = "com_github_google_googletest",
    sha256 = "3e91944af2d909a79f18ee9760765624810146ccfae8f1a8f990037a1677d44b",
    strip_prefix = "googletest-ac7a126f39d5bcd909b78c9e69900c76659b1bbb",
    urls = [
        "https://github.com/google/googletest/archive/ac7a126f39d5bcd909b78c9e69900c76659b1bbb.zip",
    ],
)

# abseil-cpp
# https://github.com/abseil/abseil-cpp
http_archive(
    name = "com_google_absl",
    sha256 = "dd8f50452b813d63ae14ec0ca046a3a12ca13f26cc756d09f0a2209b1d2378a2",
    strip_prefix = "abseil-cpp-1d07cfede2d0153ebfa23543ffcc08faf55b4539",
    urls = [
        "https://github.com/abseil/abseil-cpp/archive/1d07cfede2d0153ebfa23543ffcc08faf55b4539.zip",
    ],
)

# BoringSSL
# https://github.com/google/boringssl
http_archive(
    name = "boringssl",
    sha256 = "88e4330f4f65ebfdf24847e4807c25f3eacfd5bf1a93f6629d3941196ff9b0b3",
    strip_prefix = "boringssl-6347808f2a480a3792148bf7732232229db9b909",
    urls = [
        "https://github.com/google/boringssl/archive/6347808f2a480a3792148bf7732232229db9b909.zip",
    ],
)

# Benchmarks
# https://github.com/google/benchmark
http_archive(
    name = "com_github_google_benchmark",
    sha256 = "5f98b44165f3250f1d749b728018318d654f763ea0f4d7ea156e10e6e0cc678a",
    strip_prefix = "benchmark-5e78bedfb07c615edb2b646d1e354980268c1728",
    urls = [
        "https://github.com/google/benchmark/archive/5e78bedfb07c615edb2b646d1e354980268c1728.zip",
    ],
)

# IREE for cc_embed_data.
# https://github.com/google/iree
http_archive(
    name = "com_github_google_iree",
    sha256 = "aa369b29a5c45ae9d7aa8bf49ea1308221d1711277222f0755df6e0a575f6879",
    strip_prefix = "iree-7e6012468cbaafaaf30302748a2943771b40e2c3",
    urls = [
        "https://github.com/google/iree/archive/7e6012468cbaafaaf30302748a2943771b40e2c3.zip",
    ],
)

# riegeli for file IO
# https://github.com/google/riegeli
http_archive(
    name = "com_github_google_riegeli",
    sha256 = "3de21a222271a1e2c5d728e7f46b63ab4520da829c09ef9727a322e693c9ac18",
    strip_prefix = "riegeli-43b7ef9f995469609b6ab07f6becc82186314bfb",
    urls = [
        "https://github.com/google/riegeli/archive/43b7ef9f995469609b6ab07f6becc82186314bfb.zip",
    ],
)

# rules_license needed for license() rule
# https://github.com/bazelbuild/rules_license
http_archive(
    name = "rules_license",
    sha256 = "6157e1e68378532d0241ecd15d3c45f6e5cfd98fc10846045509fb2a7cc9e381",
    urls = [
        "https://github.com/bazelbuild/rules_license/releases/download/0.0.4/rules_license-0.0.4.tar.gz",
    ],
)

# Highway for SIMD operations.
# https://github.com/google/highway
http_archive(
    name = "com_github_google_highway",
    sha256 = "83c252c7a9b8fcc36b9774778325c689e104365114a16adec0d536d47cb99e5f",
    strip_prefix = "highway-1c8250ed008f4ca22f2bb9edb6b75a73d9c587ff",
    urls = [
        "https://github.com/google/highway/archive/1c8250ed008f4ca22f2bb9edb6b75a73d9c587ff.zip",
    ],
)

# cppitertools for logging
# https://github.com/ryanhaining/cppitertools
http_archive(
    name = "com_github_ryanhaining_cppitertools",
    sha256 = "1608ddbe3c12b0c6e653b992ff63b5dceab9af5347ad93be8714d05e5dc17afb",
    add_prefix = "cppitertools",
    strip_prefix = "cppitertools-add5acc932dea2c78acd80747bab71ec0b5bce27",
    urls = [
        "https://github.com/ryanhaining/cppitertools/archive/add5acc932dea2c78acd80747bab71ec0b5bce27.zip",
    ],
)

# Tink for hybrid encryption.
http_archive(
    name = "tink_cc",
    sha256 = "b7d2d13345fb2c6b45daeabe1be8fc8a5ba76c35b35e8b0a46d7b724febe3ad1",
    strip_prefix = "tink-6f74b99a2bfe6677e3670799116a57268fd067fa/cc",
    urls = [
        "https://github.com/google/tink/archive/6f74b99a2bfe6677e3670799116a57268fd067fa.zip",
    ],
)

load("@tink_cc//:tink_cc_deps.bzl", "tink_cc_deps")
tink_cc_deps()

load("@tink_cc//:tink_cc_deps_init.bzl", "tink_cc_deps_init")
tink_cc_deps_init()

# Farmhash.
# https://github.com/google/farmhash
http_archive(
    name = "com_github_google_farmhash",
    build_file = "@//:bazel/farmhash.BUILD",
    sha256 = "470e87745d1393cc2793f49e9bfbd2c2cf282feeeb0c367f697996fa7e664fc5",
    add_prefix = "farmhash",
    strip_prefix = "farmhash-0d859a811870d10f53a594927d0d0b97573ad06d/src",
    urls = [
        "https://github.com/google/farmhash/archive/0d859a811870d10f53a594927d0d0b97573ad06d.zip",
    ],
)

# gflags needed for glog.
# https://github.com/gflags/gflags
http_archive(
    name = "com_github_gflags_gflags",
    sha256 = "017e0a91531bfc45be9eaf07e4d8fed33c488b90b58509dbd2e33a33b2648ae6",
    strip_prefix = "gflags-a738fdf9338412f83ab3f26f31ac11ed3f3ec4bd",
    urls = [
        "https://github.com/gflags/gflags/archive/a738fdf9338412f83ab3f26f31ac11ed3f3ec4bd.zip",
    ],
)

# glog needed by SHELL
# https://github.com/google/glog
http_archive(
    name = "com_github_google_glog",
    sha256 = "0f91ee6cc1edc3b1c53a286382e69a37e5d172ce208b7e5b305be8770d8c21b1",
    strip_prefix = "glog-f545ff5e7d7f3df95f6e86c8cb987d9d9d4bd481",
    urls = [
        "https://github.com/google/glog/archive/f545ff5e7d7f3df95f6e86c8cb987d9d9d4bd481.zip",
    ],
)

# SHELL for uint256.
# https://github.com/google/shell-encryption
http_archive(
    name = "com_github_google_shell-encryption",
    sha256 = "6b524ea06a88163f253ecd1e3f8368596d891ba78a92236c166aead90d7b5660",
    strip_prefix = "shell-encryption-cd1721d1ee9e20be16954f8161b0dbc051af4399",
    urls = [
        "https://github.com/google/shell-encryption/archive/cd1721d1ee9e20be16954f8161b0dbc051af4399.zip",
    ],
)
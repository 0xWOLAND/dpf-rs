package(default_visibility = ["//visibility:public"])

cc_library(
    name = "dpf_server",
    srcs = ["server.cc"],
    hdrs = ["server.h", "base64_utils.h", "status.h"],
    deps = [
        "@google_dpf//pir:private_information_retrieval_cc_proto",
        "@google_dpf//pir:dense_dpf_pir_database",
        "@google_dpf//pir:dense_dpf_pir_server",
        "@nlohmann_json//:json",
    ],
)

cc_library(
    name = "dpf_client",
    srcs = ["client.cc"],
    hdrs = ["client.h", "base64_utils.h", "status.h"],
    deps = [
        "@google_dpf//pir:private_information_retrieval_cc_proto",
        "@google_dpf//pir:dense_dpf_pir_client",
        "@nlohmann_json//:json",
    ],
)

cc_library(
    name = "base64_utils",
    srcs = ["base64_utils.cc"],
    hdrs = ["base64_utils.h"],
)

cc_test(
    name = "dpf_pir_test",
    srcs = ["dpf_test.cc"],
    deps = [
        ":dpf_server",
        ":dpf_client",
        ":base64_utils",
        "@com_google_googletest//:gtest_main",
        "@nlohmann_json//:json",
    ],
)
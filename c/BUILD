package(default_visibility = ["//visibility:public"])

cc_library(
    name = "dpf_database",
    srcs = ["database.cc"],
    hdrs = ["database.h"],
    deps = [
        "@google_dpf//pir:dense_dpf_pir_database",
    ],
    copts = ["-std=c++17"],
)

cc_library(
    name = "dpf_client",
    srcs = ["client.cc"],
    hdrs = ["client.h"],
    deps = [
        "@google_dpf//pir:private_information_retrieval_cc_proto",
        "@google_dpf//pir:dense_dpf_pir_client",
    ],
    copts = ["-std=c++17"],
)

cc_library(
    name = "dpf_server",
    srcs = ["server.cc"],
    hdrs = ["server.h"],
    deps = [
        "@google_dpf//pir:dense_dpf_pir_database",
        "@google_dpf//pir:dense_dpf_pir_server",
    ],
    copts = ["-std=c++17"],
)
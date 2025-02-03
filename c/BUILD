package(default_visibility = ["//visibility:public"])

cc_library(
    name = "dpf_server",
    srcs = ["server.cc"],
    hdrs = ["server.h", "base64_utils.h"],
    deps = [
        "@google_dpf//pir/testing:request_generator",
        "@google_dpf//pir:private_information_retrieval_cc_proto",
        "@google_dpf//pir:dense_dpf_pir_database",
        "@google_dpf//pir:dense_dpf_pir_server",
        "@google_dpf//pir/testing:mock_pir_database",
        "@nlohmann_json//:json",
    ],
    copts = ["-std=c++17", "-fexceptions"],  # Ensure exceptions are enabled
    linkopts = ["-lc++"],  # Explicitly link against C++ standard library
    testonly = True
)


cc_library(
    name = "dpf_client",
    srcs = ["client.cc"],
    hdrs = ["client.h", "base64_utils.h"],
    deps = [
        "@google_dpf//pir/testing:request_generator",
        "@google_dpf//pir:private_information_retrieval_cc_proto",
        "@google_dpf//pir:dense_dpf_pir_client",
        "@nlohmann_json//:json",
    ],
    copts = ["-std=c++17", "-fexceptions"],  # Ensure exceptions are enabled
    linkopts = ["-lc++"],  # Explicitly link against C++ standard library
    testonly = True
)

# cc_library(
#     name = "dpf_server",
#     srcs = ["server.cc"],
#     hdrs = ["server.h", "common.h"],
#     deps = [
#         "@google_dpf//pir:dense_dpf_pir_database",
#         "@google_dpf//pir:dense_dpf_pir_server",
#     ],
#     copts = ["-std=c++17", "-fexceptions"],  # Ensure exceptions are enabled
#     linkopts = ["-lc++"],  # Explicitly link against C++ standard library
# )
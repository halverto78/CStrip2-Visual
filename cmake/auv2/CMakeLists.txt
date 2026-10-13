cmake_minimum_required(VERSION 3.21)

# Ensure macOS deployment target matches the minimum target
if (NOT DEFINED CMAKE_OSX_DEPLOYMENT_TARGET)
    set(CMAKE_OSX_DEPLOYMENT_TARGET "10.13" CACHE STRING "Minimum macOS version")
endif()

project(cstrip2-auv2
    LANGUAGES C CXX
    VERSION 0.1.0
    DESCRIPTION "CStrip2 AUv2 Wrapper"
)

if (APPLE)
    enable_language(OBJC)
    enable_language(OBJCXX)
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Configure paths to clap-wrapper and dependencies
if (NOT DEFINED CLAP_WRAPPER_DIR)
    set(CLAP_WRAPPER_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../clap-wrapper" CACHE PATH "Path to clap-wrapper repository")
endif()

if (NOT DEFINED CLAP_SDK_ROOT)
    set(CLAP_SDK_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../clap" CACHE PATH "Path to CLAP SDK headers")
endif()

if (NOT DEFINED AUDIOUNIT_SDK_ROOT)
    set(AUDIOUNIT_SDK_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../AudioUnitSDK" CACHE PATH "Path to Apple AudioUnitSDK")
endif()

# Include clap-wrapper as subproject
# EXCLUDE_FROM_ALL ensures clap-wrapper default targets (or tests) are not built automatically
add_subdirectory("${CLAP_WRAPPER_DIR}" clap-wrapper EXCLUDE_FROM_ALL)

if (APPLE)
    # Target for AUv2 bundle (.component)
    add_library(CStrip2_auv2 MODULE)

    # Configure AUv2 wrapper using clap-wrapper's target_add_auv2_wrapper
    target_add_auv2_wrapper(
        TARGET CStrip2_auv2
        OUTPUT_NAME "CStrip2"
        BUNDLE_IDENTIFIER "com.amr.audio.cstrip2"
        BUNDLE_VERSION "0.1.0"
        PREFER_CMAKE_AUV2_CONFIGURATION TRUE
        MANUFACTURER_NAME "AMR"
        MANUFACTURER_CODE "AMR2"
        SUBTYPE_CODE "Cs2A"
        INSTRUMENT_TYPE "aufx"
    )
else()
    message(STATUS "AUv2 is only built on Apple/macOS platforms.")
endif()

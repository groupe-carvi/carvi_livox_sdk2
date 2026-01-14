# carvi_livox_sdk2
A Rust wrapper for the Livox-SDK2.

This crate vendors and builds the upstream C++ library (https://github.com/Livox-SDK/Livox-SDK2) by default.

## Build requirements (vendored mode)

- `git`
- `cmake`
- a C++ toolchain (e.g. `g++`/`clang++`)

## Features

- `vendored` (default): clone Livox-SDK2 into `target/vendor/` and build/install into `target/vendor-build/`.
- `system`: link against an already-installed Livox-SDK2.
- `link-static` (default): link `livox_lidar_sdk_static`.
- `link-shared`: link `livox_lidar_sdk_shared`.

## Environment variables

### Vendored

- `LIVOX_SDK2_REPOSITORY` (default: `https://github.com/Livox-SDK/Livox-SDK2.git`)
- `LIVOX_SDK2_TAG` (default: `v1.2.5`)
- `LIVOX_SDK2_SOURCE` (use a pre-downloaded source tree; skips network)
- `LIVOX_SDK2_AUTO_DOWNLOAD=0|1` (default: `1`)

### System

- `LIVOX_SDK2_ROOT` (prefix containing `include/` and `lib/`)
- or: `LIVOX_SDK2_INCLUDE_DIR` and `LIVOX_SDK2_LIB_DIR`

### Link selection

- `LIVOX_SDK2_LINK=static|shared` (overrides feature selection)

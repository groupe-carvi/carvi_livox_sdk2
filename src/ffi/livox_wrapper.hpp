#pragma once

// Autocxx consumes this header as C++.
// We keep this file small and stable, and avoid default arguments in the upstream API.

#include "livox_lidar_api.h"

namespace carvi_livox_sdk2 {

// Upstream signature uses a default argument; make it explicit for bindings.
inline bool sdk_init(const char* config_path) {
  return LivoxLidarSdkInit(config_path);
}

// Explicit host-ip variant. Some devices/firmware require a non-empty host_ip
// to verify network segment and complete initialization.
inline bool sdk_init_with_host_ip(const char* config_path, const char* host_ip) {
  return LivoxLidarSdkInit(config_path, host_ip ? host_ip : "", nullptr);
}

inline void sdk_uninit() {
  LivoxLidarSdkUninit();
}

inline bool sdk_start() {
  return LivoxLidarSdkStart();
}

inline void sdk_stop() {
  // Livox-SDK2 does not provide a dedicated "stop" API (only init/start/uninit).
  // This wrapper is kept for backwards compatibility with earlier iterations
  // of this crate; it is intentionally a no-op.
}

// Wrapper for setting point cloud callback.
// Note: This is simplified; in practice, you'd need to handle the callback properly.
inline void set_point_cloud_callback(LivoxLidarPointCloudCallBack callback, void* client_data) {
  SetLivoxLidarPointCloudCallBack(callback, client_data);
}

}  // namespace carvi_livox_sdk2

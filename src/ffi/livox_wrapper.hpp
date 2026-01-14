#pragma once

// Autocxx consumes this header as C++.
// We keep this file small and stable, and avoid default arguments in the upstream API.

#include "livox_lidar_api.h"

namespace carvi_livox_sdk2 {

// Upstream signature uses a default argument; make it explicit for bindings.
inline bool sdk_init(const char* config_path) {
  return LivoxLidarSdkInit(config_path);
}

inline void sdk_uninit() {
  LivoxLidarSdkUninit();
}

inline bool sdk_start() {
  return LivoxLidarSdkStart();
}

inline void sdk_stop() {
  LivoxLidarSdkUninit();
}

// Wrapper for setting point cloud callback.
// Note: This is simplified; in practice, you'd need to handle the callback properly.
inline void set_point_cloud_callback(LivoxLidarPointCloudCallBack callback, void* client_data) {
  SetLivoxLidarPointCloudCallBack(callback, client_data);
}

}  // namespace carvi_livox_sdk2

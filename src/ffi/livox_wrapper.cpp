#include "livox_wrapper.hpp"

extern "C" void carvi_livox_sdk2_set_point_cloud_callback(LivoxLidarPointCloudCallBack cb,
																													void* client_data) {
	SetLivoxLidarPointCloudCallBack(cb, client_data);
}

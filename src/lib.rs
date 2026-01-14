#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    ffi::CStr,
    ptr,
    sync::{Mutex, OnceLock},
    sync::atomic::{AtomicUsize, Ordering},
};

#[allow(unsafe_op_in_unsafe_fn)]
pub mod bindings;

pub mod pointcloud;

/// Low-level generated bindings (autocxx).
///
/// Most users should prefer the safe-ish [`Sdk`] wrapper.
pub mod sys {
    pub use crate::bindings::{sdk_init, sdk_start, sdk_stop, sdk_uninit};

    /// Livox headers use `#pragma pack(1)`.
    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarEthernetPacket {
        pub version: u8,
        pub length: u16,
        /// Unit: 0.1 us
        pub time_interval: u16,
        pub dot_num: u16,
        pub udp_cnt: u16,
        pub frame_cnt: u8,
        pub data_type: u8,
        pub time_type: u8,
        pub rsvd: [u8; 12],
        pub crc32: u32,
        pub timestamp: [u8; 8],
        /// Point cloud data (flexible array member).
        pub data: [u8; 1],
    }

    pub type LivoxLidarPointCloudCallBack = Option<
        unsafe extern "C" fn(
            handle: u32,
            dev_type: u8,
            data: *mut LivoxLidarEthernetPacket,
            client_data: *mut std::ffi::c_void,
        ),
    >;

    unsafe extern "C" {
        fn carvi_livox_sdk2_set_point_cloud_callback(
            cb: LivoxLidarPointCloudCallBack,
            client_data: *mut std::ffi::c_void,
        );
    }

    /// Register (or clear) the global point cloud callback.
    ///
    /// # Safety
    /// - The callback will be invoked from SDK-owned threads.
    /// - The callback must be `extern "C"` and must not unwind.
    /// - `client_data` must remain valid for as long as the SDK may call back.
    pub unsafe fn set_point_cloud_callback(
        cb: LivoxLidarPointCloudCallBack,
        client_data: *mut std::ffi::c_void,
    ) {
        unsafe { carvi_livox_sdk2_set_point_cloud_callback(cb, client_data) };
    }
}

static SDK_REFCOUNT: AtomicUsize = AtomicUsize::new(0);

/// Errors that can occur during SDK initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitError {
    InitFailed,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::InitFailed => write!(f, "LivoxLidarSdkInit failed"),
        }
    }
}

impl std::error::Error for InitError {}

/// Errors that can occur during SDK start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    StartFailed,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::StartFailed => write!(f, "LivoxLidarSdkStart failed"),
        }
    }
}

impl std::error::Error for StartError {}

/// Errors that can occur during SDK stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopError {
    StopFailed,
}

impl std::fmt::Display for StopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopError::StopFailed => write!(f, "SDK stop failed"),
        }
    }
}

impl std::error::Error for StopError {}

/// Errors that can occur when installing a point cloud callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackError {
    AlreadyInstalled,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallbackError::AlreadyInstalled => write!(f, "point cloud callback already installed"),
        }
    }
}

impl std::error::Error for CallbackError {}

type PointCloudHandler = Box<dyn FnMut(pointcloud::PointCloudPacket) + Send + 'static>;

static POINTCLOUD_HANDLER: OnceLock<Mutex<Option<PointCloudHandler>>> = OnceLock::new();

fn handler_slot() -> &'static Mutex<Option<PointCloudHandler>> {
    POINTCLOUD_HANDLER.get_or_init(|| Mutex::new(None))
}

unsafe extern "C" fn pointcloud_trampoline(
    _handle: u32,
    _dev_type: u8,
    packet: *mut sys::LivoxLidarEthernetPacket,
    _client_data: *mut std::ffi::c_void,
) {
    // Must not unwind across FFI.
    let _ = std::panic::catch_unwind(|| {
        let decoded = unsafe { pointcloud::decode_packet(packet as *const sys::LivoxLidarEthernetPacket) };
        let Some(pkt) = decoded else {
            return;
        };

        if let Ok(mut guard) = handler_slot().lock() {
            if let Some(cb) = guard.as_mut() {
                cb(pkt);
            }
        }
    });
}

/// A minimal RAII guard for `LivoxLidarSdkInit`/`LivoxLidarSdkUninit`.
///
/// Notes:
/// - This does **not** configure callbacks yet.
/// - The upstream SDK owns background threads; callbacks will arrive from those threads.
pub struct Sdk {
    _private: (),
}

impl Sdk {
    /// Initialize the Livox-SDK2.
    ///
    /// Pass `None` to use the SDK default (equivalent to `nullptr`).
    pub fn init(config_path: Option<&CStr>) -> Result<Self, InitError> {
        // Fast-path: already initialized.
        let prev = SDK_REFCOUNT.fetch_add(1, Ordering::AcqRel);
        if prev > 0 {
            return Ok(Self { _private: () });
        }

        let ok = unsafe {
            let p = config_path.map(|s| s.as_ptr()).unwrap_or(ptr::null());
            sys::sdk_init(p)
        };

        if ok {
            Ok(Self { _private: () })
        } else {
            // Roll back refcount on failure.
            SDK_REFCOUNT.fetch_sub(1, Ordering::AcqRel);
            Err(InitError::InitFailed)
        }
    }

    /// Start the SDK.
    pub fn start(&self) -> Result<(), StartError> {
        let ret = sys::sdk_start();
        if ret {
            Ok(())
        } else {
            Err(StartError::StartFailed)
        }
    }

    /// Stop the SDK.
    pub fn stop(&self) -> Result<(), StopError> {
        sys::sdk_stop();
        Ok(())
    }

    /// Install a safe Rust callback which will be invoked for each decoded point cloud packet.
    ///
    /// The callback is invoked on SDK-owned background threads.
    ///
    /// Returns a guard which uninstalls the callback on drop.
    pub fn install_pointcloud_callback(
        &self,
        cb: impl FnMut(pointcloud::PointCloudPacket) + Send + 'static,
    ) -> Result<PointCloudCallbackGuard, CallbackError> {
        let slot = handler_slot();
        let mut guard = slot.lock().expect("pointcloud handler lock poisoned");
        if guard.is_some() {
            return Err(CallbackError::AlreadyInstalled);
        }
        *guard = Some(Box::new(cb));

        // Install the SDK callback.
        unsafe { sys::set_point_cloud_callback(Some(pointcloud_trampoline), ptr::null_mut()) };

        Ok(PointCloudCallbackGuard { _private: () })
    }
}

/// A guard returned by [`Sdk::install_pointcloud_callback`].
///
/// Dropping this guard unregisters the SDK callback and clears the Rust handler.
pub struct PointCloudCallbackGuard {
    _private: (),
}

impl Drop for PointCloudCallbackGuard {
    fn drop(&mut self) {
        // Clear SDK callback first to stop new calls racing with handler teardown.
        unsafe { sys::set_point_cloud_callback(None, ptr::null_mut()) };

        if let Ok(mut guard) = handler_slot().lock() {
            *guard = None;
        }
    }
}

impl Drop for Sdk {
    fn drop(&mut self) {
        let prev = SDK_REFCOUNT.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            // Last instance dropped.
            sys::sdk_uninit();
        }
    }
}

// No unit tests yet: calling the SDK in CI may require device/network/config.

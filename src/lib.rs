#![deny(unsafe_op_in_unsafe_fn)]

use std::{
    ffi::CStr,
    ptr,
    sync::{Mutex, OnceLock},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[allow(unsafe_op_in_unsafe_fn)]
pub mod bindings;

pub mod pointcloud;
pub mod imu;
pub mod cmd;

/// Low-level generated bindings (autocxx).
///
/// Most users should prefer the safe-ish [`Sdk`] wrapper.
pub mod sys {
    pub use crate::bindings::{sdk_init, sdk_init_with_host_ip, sdk_start, sdk_stop, sdk_uninit};

    use std::ffi::{c_char, c_void};

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

    #[allow(non_camel_case_types)]
    pub type livox_status = i32;

    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LivoxLidarStatus {
        SendFailed = -9,
        HandlerImplNotExist = -8,
        InvalidHandle = -7,
        ChannelNotExist = -6,
        NotEnoughMemory = -5,
        Timeout = -4,
        NotSupported = -3,
        NotConnected = -2,
        Failure = -1,
        Success = 0,
    }

    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LivoxLidarPointDataType {
        Imu = 0,
        CartesianCoordinateHigh = 0x01,
        CartesianCoordinateLow = 0x02,
        SphericalCoordinate = 0x03,
    }

    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LivoxLidarScanPattern {
        NoneRepetive = 0x00,
        Repetive = 0x01,
        RepetiveLowFrameRate = 0x02,
    }

    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LivoxLidarDetectMode {
        Normal = 0x00,
        Sensitive = 0x01,
    }

    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LivoxLidarWorkMode {
        Normal = 0x01,
        WakeUp = 0x02,
        Sleep = 0x03,
        Error = 0x04,
        PowerOnSelfTest = 0x05,
        MotorStarting = 0x06,
        MotorStoping = 0x07,
        Upgrade = 0x08,
    }

    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LivoxLidarWorkModeAfterBoot {
        Default = 0x00,
        Normal = 0x01,
        WakeUp = 0x02,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct FuncIOCfg {
        pub in0: u8,
        pub int1: u8,
        pub out0: u8,
        pub out1: u8,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarAsyncControlResponse {
        pub ret_code: u8,
        pub error_key: u16,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarResetResponse {
        pub ret_code: u8,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarRebootResponse {
        pub ret_code: u8,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarLoggerResponse {
        pub ret_code: u8,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarRmcSyncTimeResponse {
        pub ret: u8,
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarInfo {
        pub dev_type: u8,
        pub sn: [c_char; 16],
        pub lidar_ip: [c_char; 16],
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    pub struct LivoxLidarCmdPacket {
        pub sof: u8,
        pub version: u8,
        pub length: u16,
        pub seq_num: u32,
        pub cmd_id: u16,
        pub cmd_type: u8,
        pub sender_type: u8,
        pub rsvd: [c_char; 6],
        pub crc16_h: u16,
        pub crc32_d: u32,
        pub data: [u8; 1],
    }

    pub type LivoxLidarPointCloudCallBack = Option<
        unsafe extern "C" fn(
            handle: u32,
            dev_type: u8,
            data: *mut LivoxLidarEthernetPacket,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarPointCloudObserver = Option<
        unsafe extern "C" fn(
            handle: u32,
            dev_type: u8,
            data: *mut LivoxLidarEthernetPacket,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarCmdObserverCallBack = Option<
        unsafe extern "C" fn(handle: u32, data: *const LivoxLidarCmdPacket, client_data: *mut c_void),
    >;

    pub type LivoxLidarImuDataCallback = Option<
        unsafe extern "C" fn(
            handle: u32,
            dev_type: u8,
            data: *mut LivoxLidarEthernetPacket,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarInfoCallback = Option<
        unsafe extern "C" fn(handle: u32, dev_type: u8, info: *const c_char, client_data: *mut c_void),
    >;

    pub type LivoxLidarInfoChangeCallback = Option<
        unsafe extern "C" fn(handle: u32, info: *const LivoxLidarInfo, client_data: *mut c_void),
    >;

    pub type LivoxLidarAsyncControlCallback = Option<
        unsafe extern "C" fn(
            status: livox_status,
            handle: u32,
            response: *mut LivoxLidarAsyncControlResponse,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarResetCallback = Option<
        unsafe extern "C" fn(
            status: livox_status,
            handle: u32,
            response: *mut LivoxLidarResetResponse,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarRebootCallback = Option<
        unsafe extern "C" fn(
            status: livox_status,
            handle: u32,
            response: *mut LivoxLidarRebootResponse,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarLoggerCallback = Option<
        unsafe extern "C" fn(
            status: livox_status,
            handle: u32,
            response: *mut LivoxLidarLoggerResponse,
            client_data: *mut c_void,
        ),
    >;

    pub type LivoxLidarRmcSyncTimeCallBack = Option<
        unsafe extern "C" fn(
            status: livox_status,
            handle: u32,
            data: *mut LivoxLidarRmcSyncTimeResponse,
            client_data: *mut c_void,
        ),
    >;

    unsafe extern "C" {
        fn carvi_livox_sdk2_set_point_cloud_callback(
            cb: LivoxLidarPointCloudCallBack,
            client_data: *mut c_void,
        );

        pub fn SetLivoxLidarImuDataCallback(cb: LivoxLidarImuDataCallback, client_data: *mut c_void);
        pub fn SetLivoxLidarInfoCallback(cb: LivoxLidarInfoCallback, client_data: *mut c_void);
        pub fn SetLivoxLidarInfoChangeCallback(cb: LivoxLidarInfoChangeCallback, client_data: *mut c_void);

        pub fn LivoxLidarAddCmdObserver(cb: LivoxLidarCmdObserverCallBack, client_data: *mut c_void);
        pub fn LivoxLidarRemoveCmdObserver();

        pub fn LivoxLidarAddPointCloudObserver(cb: LivoxLidarPointCloudObserver, client_data: *mut c_void) -> u16;
        pub fn LivoxLidarRemovePointCloudObserver(id: u16);

        pub fn DisableLivoxSdkConsoleLogger();
        pub fn SaveLivoxLidarSdkLoggerFile();

        pub fn SetLivoxLidarPclDataType(
            handle: u32,
            data_type: LivoxLidarPointDataType,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarScanPattern(
            handle: u32,
            scan_pattern: LivoxLidarScanPattern,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarDualEmit(
            handle: u32,
            enable: bool,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn EnableLivoxLidarPointSend(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn DisableLivoxLidarPointSend(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn EnableLivoxLidarImuData(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn DisableLivoxLidarImuData(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn LivoxLidarRequestReset(
            handle: u32,
            cb: LivoxLidarResetCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn LivoxLidarRequestReboot(
            handle: u32,
            cb: LivoxLidarRebootCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn LivoxLidarStartLogger(
            handle: u32,
            log_type: i32,
            cb: LivoxLidarLoggerCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn LivoxLidarStopLogger(
            handle: u32,
            log_type: i32,
            cb: LivoxLidarLoggerCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarRmcSyncTime(
            handle: u32,
            rmc: *const c_char,
            rmc_length: u16,
            cb: LivoxLidarRmcSyncTimeCallBack,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn EnableLivoxLidarFov(
            handle: u32,
            fov_en: u8,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn DisableLivoxLidarFov(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarDetectMode(
            handle: u32,
            mode: LivoxLidarDetectMode,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarFuncIOCfg(
            handle: u32,
            func_io_cfg: *mut FuncIOCfg,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarBlindSpot(
            handle: u32,
            blind_spot: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarWorkMode(
            handle: u32,
            work_mode: LivoxLidarWorkMode,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn EnableLivoxLidarGlassHeat(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn DisableLivoxLidarGlassHeat(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn StartForcedHeating(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn StopForcedHeating(
            handle: u32,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;

        pub fn SetLivoxLidarWorkModeAfterBoot(
            handle: u32,
            work_mode: LivoxLidarWorkModeAfterBoot,
            cb: LivoxLidarAsyncControlCallback,
            client_data: *mut c_void,
        ) -> livox_status;
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

/// A raw Livox-SDK2 status code (`livox_status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status(pub sys::livox_status);

impl Status {
    pub fn code(self) -> i32 {
        self.0
    }

    pub fn is_success(self) -> bool {
        self.0 == sys::LivoxLidarStatus::Success as i32
    }
}

impl From<sys::livox_status> for Status {
    fn from(v: sys::livox_status) -> Self {
        Self(v)
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.0 {
            x if x == sys::LivoxLidarStatus::Success as i32 => "kLivoxLidarStatusSuccess",
            x if x == sys::LivoxLidarStatus::Failure as i32 => "kLivoxLidarStatusFailure",
            x if x == sys::LivoxLidarStatus::NotConnected as i32 => "kLivoxLidarStatusNotConnected",
            x if x == sys::LivoxLidarStatus::NotSupported as i32 => "kLivoxLidarStatusNotSupported",
            x if x == sys::LivoxLidarStatus::Timeout as i32 => "kLivoxLidarStatusTimeout",
            x if x == sys::LivoxLidarStatus::NotEnoughMemory as i32 => "kLivoxLidarStatusNotEnoughMemory",
            x if x == sys::LivoxLidarStatus::ChannelNotExist as i32 => "kLivoxLidarStatusChannelNotExist",
            x if x == sys::LivoxLidarStatus::InvalidHandle as i32 => "kLivoxLidarStatusInvalidHandle",
            x if x == sys::LivoxLidarStatus::HandlerImplNotExist as i32 => "kLivoxLidarStatusHandlerImplNotExist",
            x if x == sys::LivoxLidarStatus::SendFailed as i32 => "kLivoxLidarStatusSendFailed",
            _ => "(unknown)",
        };
        write!(f, "{name} ({})", self.0)
    }
}

impl std::error::Error for Status {}

/// Errors returned by blocking command helpers.
#[derive(Debug)]
pub enum CommandError {
    /// The SDK returned a non-success status immediately (e.g. send failed).
    SubmitFailed(Status),
    /// The async callback returned a non-success status.
    CommandFailed(Status),
    /// The device returned a non-zero `ret_code`.
    DeviceRejected { ret_code: u8, error_key: Option<u16> },
    /// Invalid argument passed to a wrapper method.
    InvalidArgument(&'static str),
    /// Timed out waiting for the async callback.
    Timeout(Duration),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::SubmitFailed(s) => write!(f, "command submit failed: {s}"),
            CommandError::CommandFailed(s) => write!(f, "command failed: {s}"),
            CommandError::DeviceRejected { ret_code, error_key } => {
                write!(f, "device rejected command: ret_code={ret_code}")?;
                if let Some(k) = error_key {
                    write!(f, ", error_key=0x{k:04x}")?;
                }
                Ok(())
            }
            CommandError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            CommandError::Timeout(d) => write!(f, "timed out waiting for callback after {d:?}"),
        }
    }
}

impl std::error::Error for CommandError {}

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

type ImuHandler = Box<dyn FnMut(imu::ImuPacket) + Send + 'static>;
static IMU_HANDLER: OnceLock<Mutex<Option<ImuHandler>>> = OnceLock::new();
fn imu_handler_slot() -> &'static Mutex<Option<ImuHandler>> {
    IMU_HANDLER.get_or_init(|| Mutex::new(None))
}

unsafe extern "C" fn imu_trampoline(
    _handle: u32,
    _dev_type: u8,
    packet: *mut sys::LivoxLidarEthernetPacket,
    _client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        let decoded = unsafe { imu::decode_packet(packet as *const sys::LivoxLidarEthernetPacket) };
        let Some(pkt) = decoded else {
            return;
        };

        if let Ok(mut guard) = imu_handler_slot().lock() {
            if let Some(cb) = guard.as_mut() {
                cb(pkt);
            }
        }
    });
}

/// A status/info message emitted by the SDK.
#[derive(Debug, Clone)]
pub struct InfoMessage {
    pub handle: u32,
    pub dev_type: u8,
    pub info: String,
}

type InfoHandler = Box<dyn FnMut(InfoMessage) + Send + 'static>;
static INFO_HANDLER: OnceLock<Mutex<Option<InfoHandler>>> = OnceLock::new();
fn info_handler_slot() -> &'static Mutex<Option<InfoHandler>> {
    INFO_HANDLER.get_or_init(|| Mutex::new(None))
}

unsafe extern "C" fn info_trampoline(
    handle: u32,
    dev_type: u8,
    info: *const std::ffi::c_char,
    _client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if info.is_null() {
            return;
        }
        let s = unsafe { CStr::from_ptr(info) }.to_string_lossy().into_owned();
        if let Ok(mut guard) = info_handler_slot().lock() {
            if let Some(cb) = guard.as_mut() {
                cb(InfoMessage { handle, dev_type, info: s });
            }
        }
    });
}

/// Device info change notification.
#[derive(Debug, Clone)]
pub struct InfoChange {
    pub handle: u32,
    pub dev_type: u8,
    pub sn: String,
    pub lidar_ip: String,
}

type InfoChangeHandler = Box<dyn FnMut(InfoChange) + Send + 'static>;
static INFO_CHANGE_HANDLER: OnceLock<Mutex<Option<InfoChangeHandler>>> = OnceLock::new();
fn info_change_handler_slot() -> &'static Mutex<Option<InfoChangeHandler>> {
    INFO_CHANGE_HANDLER.get_or_init(|| Mutex::new(None))
}

fn c_char_array_to_string(arr: &[std::ffi::c_char]) -> String {
    let bytes: Vec<u8> = arr
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

unsafe extern "C" fn info_change_trampoline(
    handle: u32,
    info: *const sys::LivoxLidarInfo,
    _client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if info.is_null() {
            return;
        }
        let i = unsafe { core::ptr::read_unaligned(info) };
        let sn = c_char_array_to_string(&i.sn);
        let lidar_ip = c_char_array_to_string(&i.lidar_ip);
        if let Ok(mut guard) = info_change_handler_slot().lock() {
            if let Some(cb) = guard.as_mut() {
                cb(InfoChange {
                    handle,
                    dev_type: i.dev_type,
                    sn,
                    lidar_ip,
                });
            }
        }
    });
}

type CmdHandler = Box<dyn FnMut(cmd::CmdPacket) + Send + 'static>;
static CMD_HANDLER: OnceLock<Mutex<Option<CmdHandler>>> = OnceLock::new();
fn cmd_handler_slot() -> &'static Mutex<Option<CmdHandler>> {
    CMD_HANDLER.get_or_init(|| Mutex::new(None))
}

unsafe extern "C" fn cmd_trampoline(
    handle: u32,
    data: *const sys::LivoxLidarCmdPacket,
    _client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        let decoded = unsafe { cmd::decode_packet(handle, data) };
        let Some(pkt) = decoded else {
            return;
        };
        if let Ok(mut guard) = cmd_handler_slot().lock() {
            if let Some(cb) = guard.as_mut() {
                cb(pkt);
            }
        }
    });
}

fn default_cmd_timeout() -> Duration {
    Duration::from_secs(2)
}

#[derive(Debug, Clone, Copy)]
struct AsyncControlResult {
    status: Status,
    ret_code: Option<u8>,
    error_key: Option<u16>,
}

struct CmdCtx<T> {
    tx: std::sync::mpsc::Sender<T>,
}

struct RmcCtx {
    tx: std::sync::mpsc::Sender<AsyncControlResult>,
    _rmc: std::ffi::CString,
}

unsafe extern "C" fn async_control_trampoline(
    status: sys::livox_status,
    _: u32,
    response: *mut sys::LivoxLidarAsyncControlResponse,
    client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if client_data.is_null() {
            return;
        }
        let ctx = unsafe { Box::from_raw(client_data as *mut CmdCtx<AsyncControlResult>) };
        let (ret_code, error_key) = if response.is_null() {
            (None, None)
        } else {
            let r = unsafe { core::ptr::read_unaligned(response) };
            (Some(r.ret_code), Some(r.error_key))
        };
        let _ = ctx.tx.send(AsyncControlResult {
            status: Status(status),
            ret_code,
            error_key,
        });
    });
}

unsafe extern "C" fn reset_trampoline(
    status: sys::livox_status,
    _: u32,
    response: *mut sys::LivoxLidarResetResponse,
    client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if client_data.is_null() {
            return;
        }
        let ctx = unsafe { Box::from_raw(client_data as *mut CmdCtx<AsyncControlResult>) };
        let ret_code = if response.is_null() {
            None
        } else {
            let r = unsafe { core::ptr::read_unaligned(response) };
            Some(r.ret_code)
        };
        let _ = ctx.tx.send(AsyncControlResult {
            status: Status(status),
            ret_code,
            error_key: None,
        });
    });
}

unsafe extern "C" fn reboot_trampoline(
    status: sys::livox_status,
    _: u32,
    response: *mut sys::LivoxLidarRebootResponse,
    client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if client_data.is_null() {
            return;
        }
        let ctx = unsafe { Box::from_raw(client_data as *mut CmdCtx<AsyncControlResult>) };
        let ret_code = if response.is_null() {
            None
        } else {
            let r = unsafe { core::ptr::read_unaligned(response) };
            Some(r.ret_code)
        };
        let _ = ctx.tx.send(AsyncControlResult {
            status: Status(status),
            ret_code,
            error_key: None,
        });
    });
}

unsafe extern "C" fn logger_trampoline(
    status: sys::livox_status,
    _: u32,
    response: *mut sys::LivoxLidarLoggerResponse,
    client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if client_data.is_null() {
            return;
        }
        let ctx = unsafe { Box::from_raw(client_data as *mut CmdCtx<AsyncControlResult>) };
        let ret_code = if response.is_null() {
            None
        } else {
            let r = unsafe { core::ptr::read_unaligned(response) };
            Some(r.ret_code)
        };
        let _ = ctx.tx.send(AsyncControlResult {
            status: Status(status),
            ret_code,
            error_key: None,
        });
    });
}

unsafe extern "C" fn rmc_sync_time_trampoline(
    status: sys::livox_status,
    _: u32,
    response: *mut sys::LivoxLidarRmcSyncTimeResponse,
    client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if client_data.is_null() {
            return;
        }
        let ctx = unsafe { Box::from_raw(client_data as *mut RmcCtx) };
        let ret_code = if response.is_null() {
            None
        } else {
            let r = unsafe { core::ptr::read_unaligned(response) };
            Some(r.ret)
        };
        let _ = ctx.tx.send(AsyncControlResult {
            status: Status(status),
            ret_code,
            error_key: None,
        });
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
        Self::init_with_host_ip(config_path, None)
    }

    /// Initialize the Livox-SDK2, explicitly providing the host IPv4 address.
    ///
    /// Some devices/firmware require a non-empty host IP for network-segment validation.
    ///
    /// - `config_path`: path to SDK config, or `None` to pass `nullptr`.
    /// - `host_ip`: your host interface IP in the same subnet as the LiDAR.
    pub fn init_with_host_ip(
        config_path: Option<&CStr>,
        host_ip: Option<&CStr>,
    ) -> Result<Self, InitError> {
        // Fast-path: already initialized.
        let prev = SDK_REFCOUNT.fetch_add(1, Ordering::AcqRel);
        if prev > 0 {
            return Ok(Self { _private: () });
        }

        let ok = unsafe {
            let p = config_path.map(|s| s.as_ptr()).unwrap_or(ptr::null());
            let hip = host_ip.map(|s| s.as_ptr()).unwrap_or(ptr::null());
            sys::sdk_init_with_host_ip(p, hip)
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
    #[deprecated(note = "Livox-SDK2 has no stop API; dropping Sdk will uninitialize it. This is a no-op kept for backwards compatibility.")]
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

    /// Install a safe Rust callback which will be invoked for each decoded IMU packet.
    ///
    /// The callback is invoked on SDK-owned background threads.
    ///
    /// Returns a guard which uninstalls the callback on drop.
    pub fn install_imu_callback(
        &self,
        cb: impl FnMut(imu::ImuPacket) + Send + 'static,
    ) -> Result<ImuCallbackGuard, CallbackError> {
        let slot = imu_handler_slot();
        let mut guard = slot.lock().expect("imu handler lock poisoned");
        if guard.is_some() {
            return Err(CallbackError::AlreadyInstalled);
        }
        *guard = Some(Box::new(cb));

        unsafe { sys::SetLivoxLidarImuDataCallback(Some(imu_trampoline), ptr::null_mut()) };

        Ok(ImuCallbackGuard { _private: () })
    }

    /// Install a safe Rust callback which will receive SDK info messages.
    pub fn install_info_callback(
        &self,
        cb: impl FnMut(InfoMessage) + Send + 'static,
    ) -> Result<InfoCallbackGuard, CallbackError> {
        let slot = info_handler_slot();
        let mut guard = slot.lock().expect("info handler lock poisoned");
        if guard.is_some() {
            return Err(CallbackError::AlreadyInstalled);
        }
        *guard = Some(Box::new(cb));

        unsafe { sys::SetLivoxLidarInfoCallback(Some(info_trampoline), ptr::null_mut()) };

        Ok(InfoCallbackGuard { _private: () })
    }

    /// Install a safe Rust callback which will receive SDK device info change notifications.
    pub fn install_info_change_callback(
        &self,
        cb: impl FnMut(InfoChange) + Send + 'static,
    ) -> Result<InfoChangeCallbackGuard, CallbackError> {
        let slot = info_change_handler_slot();
        let mut guard = slot.lock().expect("info change handler lock poisoned");
        if guard.is_some() {
            return Err(CallbackError::AlreadyInstalled);
        }
        *guard = Some(Box::new(cb));

        unsafe { sys::SetLivoxLidarInfoChangeCallback(Some(info_change_trampoline), ptr::null_mut()) };

        Ok(InfoChangeCallbackGuard { _private: () })
    }

    /// Install a safe Rust callback which will observe command packets.
    pub fn install_cmd_observer(
        &self,
        cb: impl FnMut(cmd::CmdPacket) + Send + 'static,
    ) -> Result<CmdObserverGuard, CallbackError> {
        let slot = cmd_handler_slot();
        let mut guard = slot.lock().expect("cmd observer handler lock poisoned");
        if guard.is_some() {
            return Err(CallbackError::AlreadyInstalled);
        }
        *guard = Some(Box::new(cb));

        unsafe { sys::LivoxLidarAddCmdObserver(Some(cmd_trampoline), ptr::null_mut()) };

        Ok(CmdObserverGuard { _private: () })
    }

    /// Add an additional point cloud observer.
    ///
    /// Unlike [`Sdk::install_pointcloud_callback`], multiple observers can be registered.
    /// The returned guard removes the observer on drop.
    pub fn add_pointcloud_observer(
        &self,
        cb: impl FnMut(pointcloud::PointCloudPacket) + Send + 'static,
    ) -> PointCloudObserverGuard {
        let ctx = std::sync::Arc::new(PointCloudObserverCtx {
            handler: Mutex::new(Box::new(cb)),
        });
        let ctx_ptr = std::sync::Arc::into_raw(ctx);
        let id = unsafe {
            sys::LivoxLidarAddPointCloudObserver(
                Some(pointcloud_observer_trampoline),
                ctx_ptr as *mut std::ffi::c_void,
            )
        };
        PointCloudObserverGuard { id, ctx_ptr }
    }

    /// Disable console log output from the SDK.
    pub fn disable_console_logger(&self) {
        unsafe { sys::DisableLivoxSdkConsoleLogger() };
    }

    /// Ask the SDK to flush and save its log file.
    pub fn save_logger_file(&self) {
        unsafe { sys::SaveLivoxLidarSdkLoggerFile() };
    }

    fn wait_async_control<F>(&self, timeout: Duration, submit: F) -> Result<AsyncControlResult, CommandError>
    where
        F: FnOnce(sys::LivoxLidarAsyncControlCallback, *mut std::ffi::c_void) -> sys::livox_status,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = Box::new(CmdCtx { tx });
        let ctx_ptr = Box::into_raw(ctx);

        let st = submit(Some(async_control_trampoline), ctx_ptr as *mut std::ffi::c_void);
        if st != sys::LivoxLidarStatus::Success as i32 {
            unsafe { drop(Box::from_raw(ctx_ptr)) };
            return Err(CommandError::SubmitFailed(Status(st)));
        }

        rx.recv_timeout(timeout)
            .map_err(|_| CommandError::Timeout(timeout))
    }

    fn wait_reset<F>(&self, timeout: Duration, submit: F) -> Result<AsyncControlResult, CommandError>
    where
        F: FnOnce(sys::LivoxLidarResetCallback, *mut std::ffi::c_void) -> sys::livox_status,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = Box::new(CmdCtx { tx });
        let ctx_ptr = Box::into_raw(ctx);

        let st = submit(Some(reset_trampoline), ctx_ptr as *mut std::ffi::c_void);
        if st != sys::LivoxLidarStatus::Success as i32 {
            unsafe { drop(Box::from_raw(ctx_ptr)) };
            return Err(CommandError::SubmitFailed(Status(st)));
        }

        rx.recv_timeout(timeout)
            .map_err(|_| CommandError::Timeout(timeout))
    }

    fn wait_reboot<F>(&self, timeout: Duration, submit: F) -> Result<AsyncControlResult, CommandError>
    where
        F: FnOnce(sys::LivoxLidarRebootCallback, *mut std::ffi::c_void) -> sys::livox_status,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = Box::new(CmdCtx { tx });
        let ctx_ptr = Box::into_raw(ctx);

        let st = submit(Some(reboot_trampoline), ctx_ptr as *mut std::ffi::c_void);
        if st != sys::LivoxLidarStatus::Success as i32 {
            unsafe { drop(Box::from_raw(ctx_ptr)) };
            return Err(CommandError::SubmitFailed(Status(st)));
        }

        rx.recv_timeout(timeout)
            .map_err(|_| CommandError::Timeout(timeout))
    }

    fn wait_logger<F>(&self, timeout: Duration, submit: F) -> Result<AsyncControlResult, CommandError>
    where
        F: FnOnce(sys::LivoxLidarLoggerCallback, *mut std::ffi::c_void) -> sys::livox_status,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = Box::new(CmdCtx { tx });
        let ctx_ptr = Box::into_raw(ctx);

        let st = submit(Some(logger_trampoline), ctx_ptr as *mut std::ffi::c_void);
        if st != sys::LivoxLidarStatus::Success as i32 {
            unsafe { drop(Box::from_raw(ctx_ptr)) };
            return Err(CommandError::SubmitFailed(Status(st)));
        }

        rx.recv_timeout(timeout)
            .map_err(|_| CommandError::Timeout(timeout))
    }

    fn ensure_ok(result: AsyncControlResult) -> Result<(), CommandError> {
        if !result.status.is_success() {
            return Err(CommandError::CommandFailed(result.status));
        }
        if let Some(rc) = result.ret_code {
            if rc != 0 {
                return Err(CommandError::DeviceRejected {
                    ret_code: rc,
                    error_key: result.error_key,
                });
            }
        }
        Ok(())
    }

    /// Set point cloud data type.
    pub fn set_pcl_data_type(
        &self,
        handle: u32,
        data_type: sys::LivoxLidarPointDataType,
    ) -> Result<(), CommandError> {
        self.set_pcl_data_type_with_timeout(handle, data_type, default_cmd_timeout())
    }

    pub fn set_pcl_data_type_with_timeout(
        &self,
        handle: u32,
        data_type: sys::LivoxLidarPointDataType,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarPclDataType(handle, data_type, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set scan pattern.
    pub fn set_scan_pattern(
        &self,
        handle: u32,
        scan_pattern: sys::LivoxLidarScanPattern,
    ) -> Result<(), CommandError> {
        self.set_scan_pattern_with_timeout(handle, scan_pattern, default_cmd_timeout())
    }

    pub fn set_scan_pattern_with_timeout(
        &self,
        handle: u32,
        scan_pattern: sys::LivoxLidarScanPattern,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarScanPattern(handle, scan_pattern, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Enable/disable dual emit.
    pub fn set_dual_emit(&self, handle: u32, enable: bool) -> Result<(), CommandError> {
        self.set_dual_emit_with_timeout(handle, enable, default_cmd_timeout())
    }

    pub fn set_dual_emit_with_timeout(
        &self,
        handle: u32,
        enable: bool,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarDualEmit(handle, enable, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Enable point sending.
    pub fn enable_point_send(&self, handle: u32) -> Result<(), CommandError> {
        self.enable_point_send_with_timeout(handle, default_cmd_timeout())
    }

    pub fn enable_point_send_with_timeout(
        &self,
        handle: u32,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::EnableLivoxLidarPointSend(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Disable point sending.
    pub fn disable_point_send(&self, handle: u32) -> Result<(), CommandError> {
        self.disable_point_send_with_timeout(handle, default_cmd_timeout())
    }

    pub fn disable_point_send_with_timeout(
        &self,
        handle: u32,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::DisableLivoxLidarPointSend(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Enable IMU data (device-side).
    pub fn enable_imu_data(&self, handle: u32) -> Result<(), CommandError> {
        self.enable_imu_data_with_timeout(handle, default_cmd_timeout())
    }

    pub fn enable_imu_data_with_timeout(
        &self,
        handle: u32,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::EnableLivoxLidarImuData(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Disable IMU data (device-side).
    pub fn disable_imu_data(&self, handle: u32) -> Result<(), CommandError> {
        self.disable_imu_data_with_timeout(handle, default_cmd_timeout())
    }

    pub fn disable_imu_data_with_timeout(
        &self,
        handle: u32,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::DisableLivoxLidarImuData(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Enable LiDAR FOV (with flag).
    pub fn enable_fov(&self, handle: u32, fov_en: u8) -> Result<(), CommandError> {
        self.enable_fov_with_timeout(handle, fov_en, default_cmd_timeout())
    }

    pub fn enable_fov_with_timeout(&self, handle: u32, fov_en: u8, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::EnableLivoxLidarFov(handle, fov_en, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Disable LiDAR FOV.
    pub fn disable_fov(&self, handle: u32) -> Result<(), CommandError> {
        self.disable_fov_with_timeout(handle, default_cmd_timeout())
    }

    pub fn disable_fov_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::DisableLivoxLidarFov(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set detect mode.
    pub fn set_detect_mode(&self, handle: u32, mode: sys::LivoxLidarDetectMode) -> Result<(), CommandError> {
        self.set_detect_mode_with_timeout(handle, mode, default_cmd_timeout())
    }

    pub fn set_detect_mode_with_timeout(
        &self,
        handle: u32,
        mode: sys::LivoxLidarDetectMode,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarDetectMode(handle, mode, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set functional IO configuration.
    pub fn set_func_io_cfg(&self, handle: u32, cfg: sys::FuncIOCfg) -> Result<(), CommandError> {
        self.set_func_io_cfg_with_timeout(handle, cfg, default_cmd_timeout())
    }

    pub fn set_func_io_cfg_with_timeout(
        &self,
        handle: u32,
        cfg: sys::FuncIOCfg,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let mut cfg = cfg;
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarFuncIOCfg(handle, &mut cfg as *mut sys::FuncIOCfg, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set blind spot.
    pub fn set_blind_spot(&self, handle: u32, blind_spot: u32) -> Result<(), CommandError> {
        self.set_blind_spot_with_timeout(handle, blind_spot, default_cmd_timeout())
    }

    pub fn set_blind_spot_with_timeout(
        &self,
        handle: u32,
        blind_spot: u32,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarBlindSpot(handle, blind_spot, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set device work mode.
    pub fn set_work_mode(&self, handle: u32, work_mode: sys::LivoxLidarWorkMode) -> Result<(), CommandError> {
        self.set_work_mode_with_timeout(handle, work_mode, default_cmd_timeout())
    }

    pub fn set_work_mode_with_timeout(
        &self,
        handle: u32,
        work_mode: sys::LivoxLidarWorkMode,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarWorkMode(handle, work_mode, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Enable glass heating.
    pub fn enable_glass_heat(&self, handle: u32) -> Result<(), CommandError> {
        self.enable_glass_heat_with_timeout(handle, default_cmd_timeout())
    }

    pub fn enable_glass_heat_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::EnableLivoxLidarGlassHeat(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Disable glass heating.
    pub fn disable_glass_heat(&self, handle: u32) -> Result<(), CommandError> {
        self.disable_glass_heat_with_timeout(handle, default_cmd_timeout())
    }

    pub fn disable_glass_heat_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::DisableLivoxLidarGlassHeat(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Start forced heating (not supported on all devices).
    pub fn start_forced_heating(&self, handle: u32) -> Result<(), CommandError> {
        self.start_forced_heating_with_timeout(handle, default_cmd_timeout())
    }

    pub fn start_forced_heating_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::StartForcedHeating(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Stop forced heating (not supported on all devices).
    pub fn stop_forced_heating(&self, handle: u32) -> Result<(), CommandError> {
        self.stop_forced_heating_with_timeout(handle, default_cmd_timeout())
    }

    pub fn stop_forced_heating_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::StopForcedHeating(handle, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set the work mode used after boot.
    pub fn set_work_mode_after_boot(
        &self,
        handle: u32,
        work_mode: sys::LivoxLidarWorkModeAfterBoot,
    ) -> Result<(), CommandError> {
        self.set_work_mode_after_boot_with_timeout(handle, work_mode, default_cmd_timeout())
    }

    pub fn set_work_mode_after_boot_with_timeout(
        &self,
        handle: u32,
        work_mode: sys::LivoxLidarWorkModeAfterBoot,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_async_control(timeout, |cb, cd| unsafe {
            sys::SetLivoxLidarWorkModeAfterBoot(handle, work_mode, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Request a device reset.
    pub fn request_reset(&self, handle: u32) -> Result<(), CommandError> {
        self.request_reset_with_timeout(handle, default_cmd_timeout())
    }

    pub fn request_reset_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_reset(timeout, |cb, cd| unsafe { sys::LivoxLidarRequestReset(handle, cb, cd) })?;
        Self::ensure_ok(res)
    }

    /// Request a device reboot.
    pub fn request_reboot(&self, handle: u32) -> Result<(), CommandError> {
        self.request_reboot_with_timeout(handle, default_cmd_timeout())
    }

    pub fn request_reboot_with_timeout(&self, handle: u32, timeout: Duration) -> Result<(), CommandError> {
        let res = self.wait_reboot(timeout, |cb, cd| unsafe { sys::LivoxLidarRequestReboot(handle, cb, cd) })?;
        Self::ensure_ok(res)
    }

    pub fn start_logger(&self, handle: u32, log_type: LogType) -> Result<(), CommandError> {
        self.start_logger_with_timeout(handle, log_type, default_cmd_timeout())
    }

    pub fn start_logger_with_timeout(
        &self,
        handle: u32,
        log_type: LogType,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_logger(timeout, |cb, cd| unsafe {
            sys::LivoxLidarStartLogger(handle, log_type as i32, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    pub fn stop_logger(&self, handle: u32, log_type: LogType) -> Result<(), CommandError> {
        self.stop_logger_with_timeout(handle, log_type, default_cmd_timeout())
    }

    pub fn stop_logger_with_timeout(
        &self,
        handle: u32,
        log_type: LogType,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        let res = self.wait_logger(timeout, |cb, cd| unsafe {
            sys::LivoxLidarStopLogger(handle, log_type as i32, cb, cd)
        })?;
        Self::ensure_ok(res)
    }

    /// Set GPS "$GPRMC" string to synchronize device time.
    pub fn set_rmc_sync_time(&self, handle: u32, rmc: &str) -> Result<(), CommandError> {
        self.set_rmc_sync_time_with_timeout(handle, rmc, default_cmd_timeout())
    }

    pub fn set_rmc_sync_time_with_timeout(
        &self,
        handle: u32,
        rmc: &str,
        timeout: Duration,
    ) -> Result<(), CommandError> {
        if rmc.as_bytes().iter().any(|&b| b == 0) {
            return Err(CommandError::InvalidArgument("rmc contains NUL byte"));
        }
        let c = std::ffi::CString::new(rmc).expect("checked for NUL");
        let len = rmc.as_bytes().len();
        let len_u16: u16 = len
            .try_into()
            .map_err(|_| CommandError::InvalidArgument("rmc too long"))?;

        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = Box::new(RmcCtx { tx, _rmc: c });
        let rmc_ptr = ctx._rmc.as_ptr();
        let ctx_ptr = Box::into_raw(ctx);

        let st = unsafe {
            sys::SetLivoxLidarRmcSyncTime(
                handle,
                rmc_ptr,
                len_u16,
                Some(rmc_sync_time_trampoline),
                ctx_ptr as *mut std::ffi::c_void,
            )
        };
        if st != sys::LivoxLidarStatus::Success as i32 {
            unsafe { drop(Box::from_raw(ctx_ptr)) };
            return Err(CommandError::SubmitFailed(Status(st)));
        }

        let res = rx
            .recv_timeout(timeout)
            .map_err(|_| CommandError::Timeout(timeout))?;
        Self::ensure_ok(res)
    }
}

/// Logger type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogType {
    RealTime = 0,
    Exception = 0x01,
}

struct PointCloudObserverCtx {
    handler: Mutex<Box<dyn FnMut(pointcloud::PointCloudPacket) + Send + 'static>>,
}

unsafe extern "C" fn pointcloud_observer_trampoline(
    _handle: u32,
    _dev_type: u8,
    packet: *mut sys::LivoxLidarEthernetPacket,
    client_data: *mut std::ffi::c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if client_data.is_null() {
            return;
        }
        let decoded = unsafe { pointcloud::decode_packet(packet as *const sys::LivoxLidarEthernetPacket) };
        let Some(pkt) = decoded else {
            return;
        };

        // Hold a strong ref for the duration of this callback.
        let arc = unsafe { std::sync::Arc::from_raw(client_data as *const PointCloudObserverCtx) };
        let arc2 = arc.clone();
        std::mem::forget(arc);

        if let Ok(mut guard) = arc2.handler.lock() {
            (&mut *guard)(pkt);
        }
    });
}

/// A guard returned by [`Sdk::add_pointcloud_observer`].
///
/// Dropping this guard removes the observer and releases its callback.
pub struct PointCloudObserverGuard {
    id: u16,
    ctx_ptr: *const PointCloudObserverCtx,
}

impl Drop for PointCloudObserverGuard {
    fn drop(&mut self) {
        unsafe { sys::LivoxLidarRemovePointCloudObserver(self.id) };
        unsafe { drop(std::sync::Arc::from_raw(self.ctx_ptr)) };
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

/// A guard returned by [`Sdk::install_imu_callback`].
pub struct ImuCallbackGuard {
    _private: (),
}

impl Drop for ImuCallbackGuard {
    fn drop(&mut self) {
        unsafe { sys::SetLivoxLidarImuDataCallback(None, ptr::null_mut()) };
        if let Ok(mut guard) = imu_handler_slot().lock() {
            *guard = None;
        }
    }
}

/// A guard returned by [`Sdk::install_info_callback`].
pub struct InfoCallbackGuard {
    _private: (),
}

impl Drop for InfoCallbackGuard {
    fn drop(&mut self) {
        unsafe { sys::SetLivoxLidarInfoCallback(None, ptr::null_mut()) };
        if let Ok(mut guard) = info_handler_slot().lock() {
            *guard = None;
        }
    }
}

/// A guard returned by [`Sdk::install_info_change_callback`].
pub struct InfoChangeCallbackGuard {
    _private: (),
}

impl Drop for InfoChangeCallbackGuard {
    fn drop(&mut self) {
        unsafe { sys::SetLivoxLidarInfoChangeCallback(None, ptr::null_mut()) };
        if let Ok(mut guard) = info_change_handler_slot().lock() {
            *guard = None;
        }
    }
}

/// A guard returned by [`Sdk::install_cmd_observer`].
pub struct CmdObserverGuard {
    _private: (),
}

impl Drop for CmdObserverGuard {
    fn drop(&mut self) {
        unsafe { sys::LivoxLidarRemoveCmdObserver() };
        if let Ok(mut guard) = cmd_handler_slot().lock() {
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

//! Safe(ish) parsing utilities and types for Livox-SDK2 IMU packets.

use crate::sys;

/// A single IMU sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImuSample {
    pub gyro_x: f32,
    pub gyro_y: f32,
    pub gyro_z: f32,
    pub acc_x: f32,
    pub acc_y: f32,
    pub acc_z: f32,
}

/// A decoded IMU packet.
#[derive(Debug, Clone)]
pub struct ImuPacket {
    /// Number of samples decoded.
    pub samples: Vec<ImuSample>,
    /// Raw 8-byte timestamp from the SDK packet.
    pub timestamp: [u8; 8],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct ImuRawPoint {
    gyro_x: f32,
    gyro_y: f32,
    gyro_z: f32,
    acc_x: f32,
    acc_y: f32,
    acc_z: f32,
}

/// Decode an IMU packet into owned samples.
///
/// Returns `None` if the packet is null or the payload can't be decoded.
///
/// # Safety
/// `packet` must be a valid pointer provided by the Livox SDK callback for the duration
/// of this call.
pub unsafe fn decode_packet(packet: *const sys::LivoxLidarEthernetPacket) -> Option<ImuPacket> {
    if packet.is_null() {
        return None;
    }

    // Header is packed; pointer may be unaligned.
    let pkt = unsafe { core::ptr::read_unaligned(packet) };

    let header_bytes = core::mem::size_of::<sys::LivoxLidarEthernetPacket>() - 1;
    let total_len = pkt.length as usize;
    if total_len < header_bytes {
        return None;
    }

    let payload_len = total_len - header_bytes;
    let payload_ptr = unsafe { (packet as *const u8).add(header_bytes) };

    // IMU uses data_type == 0.
    if pkt.data_type != 0 {
        return None;
    }

    let dot_num = pkt.dot_num as usize;
    let sample_size = core::mem::size_of::<ImuRawPoint>();
    if sample_size == 0 {
        return None;
    }

    let max_count = payload_len / sample_size;
    let count = core::cmp::min(dot_num, max_count);

    if count == 0 {
        return None;
    }

    let mut samples = Vec::with_capacity(count);
    for i in 0..count {
        let p_ptr = unsafe { payload_ptr.add(i * sample_size) as *const ImuRawPoint };
        let p = unsafe { core::ptr::read_unaligned(p_ptr) };
        samples.push(ImuSample {
            gyro_x: p.gyro_x,
            gyro_y: p.gyro_y,
            gyro_z: p.gyro_z,
            acc_x: p.acc_x,
            acc_y: p.acc_y,
            acc_z: p.acc_z,
        });
    }

    Some(ImuPacket {
        samples,
        timestamp: pkt.timestamp,
    })
}

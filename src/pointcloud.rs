//! Safe(ish) parsing utilities and types for Livox-SDK2 point cloud packets.

use crate::sys;

/// Point data type in a `LivoxLidarEthernetPacket`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointDataType {
    Imu,
    CartesianHigh,
    CartesianLow,
    Spherical,
    Unknown(u8),
}

impl From<u8> for PointDataType {
    fn from(v: u8) -> Self {
        match v {
            0x00 => Self::Imu,
            0x01 => Self::CartesianHigh,
            0x02 => Self::CartesianLow,
            0x03 => Self::Spherical,
            other => Self::Unknown(other),
        }
    }
}

/// A single point in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub reflectivity: u8,
    pub tag: u8,
}

/// A decoded packet of points.
#[derive(Debug, Clone)]
pub struct PointCloudPacket {
    pub data_type: PointDataType,
    /// Number of points decoded.
    pub points: Vec<Point>,
    /// Raw 8-byte timestamp from the SDK packet.
    pub timestamp: [u8; 8],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct CartesianHighRawPoint {
    x: i32, // mm
    y: i32,
    z: i32,
    reflectivity: u8,
    tag: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct CartesianLowRawPoint {
    x: i16, // cm
    y: i16,
    z: i16,
    reflectivity: u8,
    tag: u8,
}

/// Decode a packet into owned points.
///
/// Returns `None` if the packet is null or the payload can't be decoded.
///
/// # Safety
/// `packet` must be a valid pointer provided by the Livox SDK callback for the duration
/// of this call.
pub unsafe fn decode_packet(packet: *const sys::LivoxLidarEthernetPacket) -> Option<PointCloudPacket> {
    if packet.is_null() {
        return None;
    }

    // Header is packed; pointer may be unaligned.
    let pkt = unsafe { core::ptr::read_unaligned(packet) };

    // Packet payload starts at `data[0]` (flexible array member).
    let header_bytes = core::mem::size_of::<sys::LivoxLidarEthernetPacket>() - 1;
    let total_len = pkt.length as usize;
    if total_len < header_bytes {
        return None;
    }
    let payload_len = total_len - header_bytes;
    let payload_ptr = unsafe { (packet as *const u8).add(header_bytes) };

    let data_type: PointDataType = pkt.data_type.into();
    if data_type == PointDataType::Imu {
        return None;
    }

    let dot_num = pkt.dot_num as usize;

    let mut points: Vec<Point> = Vec::new();

    match data_type {
        PointDataType::CartesianHigh => {
            let point_size = core::mem::size_of::<CartesianHighRawPoint>();
            if point_size == 0 {
                return None;
            }
            let max_count = payload_len / point_size;
            let count = core::cmp::min(dot_num, max_count);
            points.reserve(count);
            for i in 0..count {
                let p_ptr = unsafe { payload_ptr.add(i * point_size) as *const CartesianHighRawPoint };
                let p = unsafe { core::ptr::read_unaligned(p_ptr) };
                points.push(Point {
                    x: p.x as f32 / 1000.0,
                    y: p.y as f32 / 1000.0,
                    z: p.z as f32 / 1000.0,
                    reflectivity: p.reflectivity,
                    tag: p.tag,
                });
            }
        }
        PointDataType::CartesianLow => {
            let point_size = core::mem::size_of::<CartesianLowRawPoint>();
            if point_size == 0 {
                return None;
            }
            let max_count = payload_len / point_size;
            let count = core::cmp::min(dot_num, max_count);
            points.reserve(count);
            for i in 0..count {
                let p_ptr = unsafe { payload_ptr.add(i * point_size) as *const CartesianLowRawPoint };
                let p = unsafe { core::ptr::read_unaligned(p_ptr) };
                points.push(Point {
                    x: p.x as f32 / 100.0,
                    y: p.y as f32 / 100.0,
                    z: p.z as f32 / 100.0,
                    reflectivity: p.reflectivity,
                    tag: p.tag,
                });
            }
        }
        PointDataType::Spherical => {
            // Not supported yet.
            return None;
        }
        PointDataType::Unknown(_) => return None,
        PointDataType::Imu => unreachable!(),
    }

    if points.is_empty() {
        return None;
    }

    Some(PointCloudPacket {
        data_type,
        points,
        timestamp: pkt.timestamp,
    })
}

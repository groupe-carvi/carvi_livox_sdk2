//! Safe(ish) parsing utilities and types for Livox-SDK2 command packets.

use crate::sys;

/// A decoded command packet.
#[derive(Debug, Clone)]
pub struct CmdPacket {
    pub handle: u32,
    pub version: u8,
    pub seq_num: u32,
    pub cmd_id: u16,
    pub cmd_type: u8,
    pub sender_type: u8,
    pub payload: Vec<u8>,
}

/// Decode a command packet into an owned structure.
///
/// Returns `None` if the packet is null or malformed.
///
/// # Safety
/// `packet` must be a valid pointer provided by the Livox SDK callback for the duration
/// of this call.
pub unsafe fn decode_packet(handle: u32, packet: *const sys::LivoxLidarCmdPacket) -> Option<CmdPacket> {
    if packet.is_null() {
        return None;
    }

    // Header is packed; pointer may be unaligned.
    let pkt = unsafe { core::ptr::read_unaligned(packet) };

    let header_bytes = core::mem::size_of::<sys::LivoxLidarCmdPacket>() - 1;
    let total_len = pkt.length as usize;
    if total_len < header_bytes {
        return None;
    }

    let payload_len = total_len - header_bytes;
    let payload_ptr = unsafe { (packet as *const u8).add(header_bytes) };
    let payload = unsafe { core::slice::from_raw_parts(payload_ptr, payload_len) }.to_vec();

    Some(CmdPacket {
        handle,
        version: pkt.version,
        seq_num: pkt.seq_num,
        cmd_id: pkt.cmd_id,
        cmd_type: pkt.cmd_type,
        sender_type: pkt.sender_type,
        payload,
    })
}

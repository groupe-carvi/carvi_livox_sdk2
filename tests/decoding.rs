use carvi_livox_sdk2::{cmd, imu, pointcloud, sys};

fn header_bytes_ethernet() -> usize {
    core::mem::size_of::<sys::LivoxLidarEthernetPacket>() - 1
}

fn header_bytes_cmd() -> usize {
    core::mem::size_of::<sys::LivoxLidarCmdPacket>() - 1
}

#[test]
fn decode_cartesian_low_packet() {
    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    struct CartesianLowRawPoint {
        x: i16,
        y: i16,
        z: i16,
        reflectivity: u8,
        tag: u8,
    }

    let pts = [
        CartesianLowRawPoint {
            x: 100,
            y: -50,
            z: 25,
            reflectivity: 7,
            tag: 1,
        },
        CartesianLowRawPoint {
            x: -200,
            y: 0,
            z: 300,
            reflectivity: 9,
            tag: 2,
        },
    ];

    let payload_len = core::mem::size_of_val(&pts);
    let hdr = header_bytes_ethernet();

    let mut header = sys::LivoxLidarEthernetPacket {
        version: 0,
        length: (hdr + payload_len) as u16,
        time_interval: 0,
        dot_num: pts.len() as u16,
        udp_cnt: 0,
        frame_cnt: 0,
        data_type: 0x02, // CartesianLow
        time_type: 0,
        rsvd: [0u8; 12],
        crc32: 0,
        timestamp: [1, 2, 3, 4, 5, 6, 7, 8],
        data: [0u8; 1],
    };

    let mut buf = vec![0u8; hdr + payload_len];
    unsafe {
        core::ptr::copy_nonoverlapping((&mut header) as *mut _ as *const u8, buf.as_mut_ptr(), hdr);
        core::ptr::copy_nonoverlapping(
            pts.as_ptr() as *const u8,
            buf.as_mut_ptr().add(hdr),
            payload_len,
        );
    }

    let decoded = unsafe { pointcloud::decode_packet(buf.as_ptr() as *const sys::LivoxLidarEthernetPacket) }
        .expect("expected some points");

    assert_eq!(decoded.points.len(), 2);
    assert_eq!(decoded.timestamp, [1, 2, 3, 4, 5, 6, 7, 8]);

    let p0 = decoded.points[0];
    assert!((p0.x - 1.0).abs() < 1e-6);
    assert!((p0.y - (-0.5)).abs() < 1e-6);
    assert!((p0.z - 0.25).abs() < 1e-6);
    assert_eq!(p0.reflectivity, 7);
    assert_eq!(p0.tag, 1);
}

#[test]
fn decode_imu_packet() {
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

    let pts = [ImuRawPoint {
        gyro_x: 1.0,
        gyro_y: 2.0,
        gyro_z: 3.0,
        acc_x: 4.0,
        acc_y: 5.0,
        acc_z: 6.0,
    }];

    let payload_len = core::mem::size_of_val(&pts);
    let hdr = header_bytes_ethernet();

    let mut header = sys::LivoxLidarEthernetPacket {
        version: 0,
        length: (hdr + payload_len) as u16,
        time_interval: 0,
        dot_num: pts.len() as u16,
        udp_cnt: 0,
        frame_cnt: 0,
        data_type: 0x00, // IMU
        time_type: 0,
        rsvd: [0u8; 12],
        crc32: 0,
        timestamp: [8, 7, 6, 5, 4, 3, 2, 1],
        data: [0u8; 1],
    };

    let mut buf = vec![0u8; hdr + payload_len];
    unsafe {
        core::ptr::copy_nonoverlapping((&mut header) as *mut _ as *const u8, buf.as_mut_ptr(), hdr);
        core::ptr::copy_nonoverlapping(
            pts.as_ptr() as *const u8,
            buf.as_mut_ptr().add(hdr),
            payload_len,
        );
    }

    let decoded = unsafe { imu::decode_packet(buf.as_ptr() as *const sys::LivoxLidarEthernetPacket) }
        .expect("expected imu samples");

    assert_eq!(decoded.samples.len(), 1);
    assert_eq!(decoded.timestamp, [8, 7, 6, 5, 4, 3, 2, 1]);
    let s = decoded.samples[0];
    assert_eq!(s.gyro_x, 1.0);
    assert_eq!(s.gyro_y, 2.0);
    assert_eq!(s.gyro_z, 3.0);
    assert_eq!(s.acc_x, 4.0);
    assert_eq!(s.acc_y, 5.0);
    assert_eq!(s.acc_z, 6.0);
}

#[test]
fn decode_cmd_packet() {
    let payload: [u8; 4] = [10, 20, 30, 40];

    let hdr = header_bytes_cmd();
    let total_len = hdr + payload.len();

    let mut header = sys::LivoxLidarCmdPacket {
        sof: 0xAA,
        version: 1,
        length: total_len as u16,
        seq_num: 42,
        cmd_id: 0x1234,
        cmd_type: 2,
        sender_type: 3,
        rsvd: [0; 6],
        crc16_h: 0,
        crc32_d: 0,
        data: [0u8; 1],
    };

    let mut buf = vec![0u8; total_len];
    unsafe {
        core::ptr::copy_nonoverlapping((&mut header) as *mut _ as *const u8, buf.as_mut_ptr(), hdr);
        core::ptr::copy_nonoverlapping(payload.as_ptr(), buf.as_mut_ptr().add(hdr), payload.len());
    }

    let decoded = unsafe { cmd::decode_packet(99, buf.as_ptr() as *const sys::LivoxLidarCmdPacket) }
        .expect("expected cmd packet");

    assert_eq!(decoded.handle, 99);
    assert_eq!(decoded.version, 1);
    assert_eq!(decoded.seq_num, 42);
    assert_eq!(decoded.cmd_id, 0x1234);
    assert_eq!(decoded.cmd_type, 2);
    assert_eq!(decoded.sender_type, 3);
    assert_eq!(decoded.payload, payload);
}

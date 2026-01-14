use anyhow::{Context, Result};
use carvi_livox_sdk2::{Sdk, sys};
use re_web_viewer_server::WebViewerServerPort;
use rerun::external::glam;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

fn detect_host_ip() -> Option<String> {
    // Best-effort: ask the OS which source IP it would use for an outbound UDP packet.
    // This often selects the "primary" interface.
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    match sock.local_addr().ok()? {
        std::net::SocketAddr::V4(v4) => Some(v4.ip().to_string()),
        std::net::SocketAddr::V6(_) => None,
    }
}

static PACKETS_SEEN: AtomicU64 = AtomicU64::new(0);
static POINTS_SEEN: AtomicU64 = AtomicU64::new(0);

fn main() -> Result<()> {
    // Start a gRPC server and use it as log sink.
    // By default this listens on 0.0.0.0:9876.
    let rec = rerun::RecordingStreamBuilder::new("Livox HAP LiDAR")
        .serve_grpc()
        .context("Failed to start gRPC server")?;

    let host_ip = std::env::var("LIVOX_HOST_IP").ok().or_else(detect_host_ip);
    let host_ip = host_ip.context(
        "Could not determine host IPv4. Set LIVOX_HOST_IP to the host IP in the LiDAR subnet.",
    )?;
    println!("Using host IP: {host_ip}");
    let host_ip_c = std::ffi::CString::new(host_ip.as_str()).context("host ip contained NUL")?;

    // Host the Rerun web-viewer (HTTP) and point it at the gRPC /proxy endpoint.
    // IMPORTANT: if your browser is not on the same machine, `localhost` won't work.
    // Use the host's reachable IP instead.
    let connect_to = format!("rerun+http://{host_ip}:9876/proxy");
    let web_port: u16 = std::env::var("RERUN_WEB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9090);
    let web_server = rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
        // Useful for remote viewing. If you only want localhost, set RERUN_BIND_IP=127.0.0.1.
        bind_ip: std::env::var("RERUN_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_owned()),
        web_port: WebViewerServerPort(web_port),
        connect_to: vec![std::env::var("RERUN_CONNECT_URL").unwrap_or(connect_to.clone())],
        open_browser: false,
        ..Default::default()
    })?;

    println!("Initializing Livox HAP LiDAR SDK...");
    println!("rerun: web viewer served at: {}", web_server.server_url());
    println!("rerun: web viewer (LAN) URL: http://{host_ip}:{web_port}");
    println!("rerun: connect URL: {connect_to}");

    // Point cloud aggregation:
    // - We receive many small packets; logging each packet makes the viewer look like it only shows
    //   a tiny slice. Instead, accumulate packets for a short duration and log one bigger cloud.
    // - Optionally, keep a rolling window of the last N aggregated frames and log them combined.
    let accumulate_ms: u64 = std::env::var("RERUN_FRAME_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let window_frames: usize = std::env::var("RERUN_WINDOW_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    let max_points: usize = std::env::var("RERUN_MAX_POINTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_500_000)
        .max(10_000);

    println!("rerun: frame aggregation: {}ms, window_frames={}, max_points={}", accumulate_ms, window_frames, max_points);

    // Load the SDK config file from env.
    // If the file contains "__HOST_IP__", we'll substitute it into a temp copy.
    let cfg_from_env = std::env::var("LIVOX_SDK_CONFIG")
        .ok()
        .or_else(|| std::env::var("LIVOX_SDK_CONFIG_PATH").ok())
        .or_else(|| std::env::var("LIVOX_CONFIG_PATH").ok());

    let cfg_input_path = cfg_from_env.context(
        "Missing Livox SDK config path. Set LIVOX_SDK_CONFIG (or LIVOX_SDK_CONFIG_PATH / LIVOX_CONFIG_PATH) to a JSON config file.",
    )?;
    let cfg_input_path = std::path::PathBuf::from(cfg_input_path);
    if !cfg_input_path.exists() {
        anyhow::bail!("Livox SDK config file does not exist: {}", cfg_input_path.display());
    }

    let cfg_contents = std::fs::read_to_string(&cfg_input_path)
        .with_context(|| format!("failed to read livox config: {}", cfg_input_path.display()))?;

    let cfg_path = if cfg_contents.contains("__HOST_IP__") {
        let cfg_json = cfg_contents.replace("__HOST_IP__", host_ip.as_str());
        let cfg_path = std::env::temp_dir().join(format!(
            "carvi_livox_sdk2_livox_config_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));
        std::fs::write(&cfg_path, cfg_json).context("failed to write temp livox config")?;
        cfg_path
    } else {
        cfg_input_path
    };

    println!("Using Livox SDK config: {}", cfg_path.display());

    let cfg_path_c = std::ffi::CString::new(cfg_path.to_string_lossy().as_bytes())
        .context("config path contained NUL")?;

    // Initialize the SDK (assuming default config)
    let sdk = Sdk::init_with_host_ip(Some(cfg_path_c.as_c_str()), Some(host_ip_c.as_c_str()))
        .context("Failed to initialize SDK")?;

    // Capture the LiDAR handle (needed for control commands).
    let (handle_tx, handle_rx) = std::sync::mpsc::channel::<u32>();
    let _info_change_guard = sdk
        .install_info_change_callback({
            let mut sent = false;
            move |chg| {
                if sent {
                    return;
                }
                sent = true;
                println!("Detected LiDAR: handle={} dev_type={} sn={} ip={}", chg.handle, chg.dev_type, chg.sn, chg.lidar_ip);
                let _ = handle_tx.send(chg.handle);
            }
        })
        .context("Failed to install info change callback")?;

    // Ship decoded packets to the main thread (avoid doing heavy work on SDK threads).
    let (pc_tx, pc_rx) = std::sync::mpsc::sync_channel::<carvi_livox_sdk2::pointcloud::PointCloudPacket>(4096);

    // Install a safe Rust callback that receives decoded points.
    let _pc_guard = sdk
        .install_pointcloud_callback({
            let pc_tx = pc_tx.clone();
            move |pkt| {
            PACKETS_SEEN.fetch_add(1, Ordering::Relaxed);
            POINTS_SEEN.fetch_add(pkt.points.len() as u64, Ordering::Relaxed);

            // Never block SDK threads.
            let _ = pc_tx.try_send(pkt);
        }
        })
        .context("Failed to install pointcloud callback")?;

    println!("Starting SDK...");
    sdk.start().context("Failed to start SDK")?;

    // Wait briefly for a handle, then enable streaming.
    let handle = handle_rx
        .recv_timeout(Duration::from_secs(5))
        .context("Timed out waiting for LiDAR detection (no handle received)")?;

    // These are typically required to actually start streaming pointcloud packets.
    sdk.set_work_mode(handle, sys::LivoxLidarWorkMode::Normal)
        .context("Failed to set work mode")?;
    sdk.set_pcl_data_type(handle, sys::LivoxLidarPointDataType::CartesianCoordinateHigh)
        .context("Failed to set pointcloud data type")?;
    sdk.enable_point_send(handle)
        .context("Failed to enable point sending")?;

    println!("SDK started. Waiting for point cloud data... (Press Ctrl+C to stop)");
    println!("View the data in Rerun Viewer");

    let mut frame_idx: i64 = 0;
    let flush_every = Duration::from_millis(accumulate_ms);
    let mut next_flush = Instant::now() + flush_every;

    // We build one "frame" by accumulating many packets into these buffers.
    let mut cur_positions: Vec<glam::Vec3> = Vec::new();
    let mut cur_colors: Vec<rerun::Color> = Vec::new();

    // And we keep a rolling window of the last N aggregated frames.
    let mut window_positions: VecDeque<Vec<glam::Vec3>> = VecDeque::with_capacity(window_frames.min(64));
    let mut window_colors: VecDeque<Vec<rerun::Color>> = VecDeque::with_capacity(window_frames.min(64));

    let mut last_stats = Instant::now();
    let mut last_packets = 0u64;
    let mut last_points = 0u64;

    loop {
        // Drain incoming packets quickly.
        while let Ok(pkt) = pc_rx.try_recv() {
            if pkt.points.is_empty() {
                continue;
            }

            // Convert to Rerun-friendly types and append to the current frame.
            cur_positions.reserve(pkt.points.len());
            cur_colors.reserve(pkt.points.len());
            for p in pkt.points {
                if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
                    continue;
                }
                cur_positions.push(glam::Vec3::new(p.x, p.y, p.z));
                cur_colors.push(rerun::Color::from_rgb(
                    p.reflectivity,
                    p.reflectivity,
                    p.reflectivity,
                ));
            }
        }

        // Periodically publish a combined cloud.
        if Instant::now() >= next_flush {
            next_flush += flush_every;

            // Finalize the current frame into the rolling window.
            if !cur_positions.is_empty() {
                window_positions.push_back(std::mem::take(&mut cur_positions));
                window_colors.push_back(std::mem::take(&mut cur_colors));
                while window_positions.len() > window_frames {
                    window_positions.pop_front();
                    window_colors.pop_front();
                }
            }

            // Flatten the rolling window (last N frames).
            let mut total = 0usize;
            for v in &window_positions {
                total = total.saturating_add(v.len());
            }

            if total > 0 {
                // Cap to a max number of points to keep the viewer responsive.
                // We keep the most recent points by skipping from the front (oldest).
                let keep = total.min(max_points);
                let skip = total.saturating_sub(keep);

                let mut positions: Vec<glam::Vec3> = Vec::with_capacity(keep);
                let mut colors: Vec<rerun::Color> = Vec::with_capacity(keep);

                let mut seen = 0usize;
                for (pos, col) in window_positions.iter().zip(window_colors.iter()) {
                    for (p, c) in pos.iter().zip(col.iter()) {
                        if seen < skip {
                            seen += 1;
                            continue;
                        }
                        positions.push(*p);
                        colors.push(*c);
                    }
                }

                rec.set_time_sequence("frame", frame_idx);
                frame_idx += 1;
                let n = positions.len();
                let _ = rec.log(
                    "lidar/points",
                    &rerun::Points3D::new(positions)
                        .with_colors(colors)
                        .with_radii(std::iter::repeat(0.02).take(n)),
                );
            }
        }

        // Stats every ~1s.
        if last_stats.elapsed() >= Duration::from_secs(1) {
            let now = Instant::now();
            let dt = now.duration_since(last_stats).as_secs_f64().max(1e-9);
            last_stats = now;

            let packets = PACKETS_SEEN.load(Ordering::Relaxed);
            let points = POINTS_SEEN.load(Ordering::Relaxed);
            let d_packets = packets.saturating_sub(last_packets);
            let d_points = points.saturating_sub(last_points);
            last_packets = packets;
            last_points = points;

            println!(
                "rx: {d_packets} pkts/s, {d_points} pts/s (total {packets} pkts, {points} pts)",
                d_packets = (d_packets as f64 / dt).round() as u64,
                d_points = (d_points as f64 / dt).round() as u64
            );
        }

        std::thread::sleep(Duration::from_millis(5));
    }

    // Note: In a real application, handle shutdown properly
}
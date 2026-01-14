use anyhow::{Context, Result};
use carvi_livox_sdk2::Sdk;
use rerun::external::glam;
use std::{sync::Mutex, time::Duration};

static RERUN_REC: Mutex<Option<rerun::RecordingStream>> = Mutex::new(None);

fn main() -> Result<()> {
    // Start a gRPC server and use it as log sink
    let rec = rerun::RecordingStreamBuilder::new("Livox HAP LiDAR")
        .serve_grpc()
        .context("Failed to start gRPC server")?;

    // Store the recording stream globally for the callback
    *RERUN_REC.lock().unwrap() = Some(rec.clone());

    // Connect the web viewer to the gRPC server and open it in the browser.
    // (The dependency is compiled with `rerun`'s `web_viewer` feature enabled.)
    let _server_guard = rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
        connect_to: vec!["rerun+http://localhost/proxy".to_owned()],
        ..Default::default()
    })?;

    println!("Web viewer available at: http://localhost:9090 (or check console output)");
    println!("Initializing Livox HAP LiDAR SDK...");

    // Initialize the SDK (assuming default config)
    let sdk = Sdk::init(None).context("Failed to initialize SDK")?;

    // Install a safe Rust callback that receives decoded points.
    let _pc_guard = sdk
        .install_pointcloud_callback(|pkt| {
            let rec = {
                // Keep the lock scope short.
                RERUN_REC.lock().unwrap().clone()
            };
            let Some(rec) = rec else { return };

            let n = pkt.points.len();
            if n == 0 {
                return;
            }

            let mut positions: Vec<glam::Vec3> = Vec::with_capacity(n);
            let mut colors: Vec<rerun::Color> = Vec::with_capacity(n);

            for p in pkt.points {
                positions.push(glam::Vec3::new(p.x, p.y, p.z));
                colors.push(rerun::Color::from_rgb(p.reflectivity, p.reflectivity, p.reflectivity));
            }

            let _ = rec.log(
                "lidar/points",
                &rerun::Points3D::new(positions)
                    .with_colors(colors)
                    .with_radii(std::iter::repeat(0.02).take(n)),
            );
        })
        .context("Failed to install pointcloud callback")?;

    println!("Starting SDK...");
    sdk.start().context("Failed to start SDK")?;

    println!("SDK started. Waiting for point cloud data... (Press Ctrl+C to stop)");
    println!("View the data in Rerun Viewer");

    // Keep the program running
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }

    // Note: In a real application, handle shutdown properly
}
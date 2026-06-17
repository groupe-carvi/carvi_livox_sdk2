use once_cell::sync::Lazy;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

static PROJECT_ROOT: Lazy<PathBuf> = Lazy::new(|| {
    PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| env::current_dir().unwrap().to_str().unwrap().to_string()),
    )
});

static TARGET_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let target_dir = env::var("CARGO_TARGET_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PROJECT_ROOT.join("target"));

    if target_dir.is_absolute() {
        target_dir
    } else {
        PROJECT_ROOT.join(target_dir)
    }
});

// A stable cache location for build-time vendored dependencies.
// This is intentionally inside `target/` so it does not pollute the repository.
static VENDOR_FOLDER_PATH: Lazy<PathBuf> = Lazy::new(|| TARGET_DIR.join("vendor"));

const LIVOX_SDK2_REPOSITORY_URL: &str = "https://github.com/Livox-SDK/Livox-SDK2.git";
const LIVOX_SDK2_TAG_DEFAULT: &str = "v1.2.5";

const LIVOX_SDK2_ROOT_ENV: &str = "LIVOX_SDK2_ROOT";
const LIVOX_SDK2_INCLUDE_ENV: &str = "LIVOX_SDK2_INCLUDE_DIR";
const LIVOX_SDK2_LIB_ENV: &str = "LIVOX_SDK2_LIB_DIR";
const LIVOX_SDK2_SOURCE_ENV: &str = "LIVOX_SDK2_SOURCE";
const LIVOX_SDK2_REPO_ENV: &str = "LIVOX_SDK2_REPOSITORY";
const LIVOX_SDK2_TAG_ENV: &str = "LIVOX_SDK2_TAG";
const LIVOX_SDK2_AUTO_DOWNLOAD_ENV: &str = "LIVOX_SDK2_AUTO_DOWNLOAD";
const LIVOX_SDK2_LINK_ENV: &str = "LIVOX_SDK2_LINK";

macro_rules! println_build {
    ($($tokens:tt)*) => {
        println!("cargo:warning=\r\x1b[32;1m   {}", format!($($tokens)*))
    };
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/bindings.rs");
    println!("cargo:rerun-if-changed=src/ffi/livox_wrapper.hpp");
    println!("cargo:rerun-if-changed=src/ffi/livox_wrapper.cpp");

    // Configuration env vars.
    for key in [
        LIVOX_SDK2_ROOT_ENV,
        LIVOX_SDK2_INCLUDE_ENV,
        LIVOX_SDK2_LIB_ENV,
        LIVOX_SDK2_SOURCE_ENV,
        LIVOX_SDK2_REPO_ENV,
        LIVOX_SDK2_TAG_ENV,
        LIVOX_SDK2_AUTO_DOWNLOAD_ENV,
        LIVOX_SDK2_LINK_ENV,
    ] {
        println!("cargo:rerun-if-env-changed={key}");
    }

    ensure_directory(&VENDOR_FOLDER_PATH);

    let link_mode = choose_link_mode();

    // Resolve include/lib directories either via system install or vendored build.
    let (include_dir, lib_dir) = if cfg!(feature = "system") {
        resolve_system_paths()
    } else if cfg!(feature = "vendored") {
        let (src_root, tag) = ensure_livox_sdk2_source();
        apply_livox_sdk2_patches(&src_root);
        track_livox_sdk2_sources(&src_root);
        build_livox_sdk2(&src_root, &tag)
    } else {
        // Safety net: default to vendored behavior.
        let (src_root, tag) = ensure_livox_sdk2_source();
        apply_livox_sdk2_patches(&src_root);
        track_livox_sdk2_sources(&src_root);
        build_livox_sdk2(&src_root, &tag)
    };

    emit_link_directives(&lib_dir, link_mode);
    build_bindings(&include_dir);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinkMode {
    Static,
    Shared,
}

fn choose_link_mode() -> LinkMode {
    // Priority: explicit env var > cargo features > default static.
    if let Ok(v) = env::var(LIVOX_SDK2_LINK_ENV) {
        match v.as_str() {
            "static" => return LinkMode::Static,
            "shared" | "dynamic" | "dylib" => return LinkMode::Shared,
            _ => {
                println_build!(
                    "Unrecognized {LIVOX_SDK2_LINK_ENV}={v:?} (expected 'static' or 'shared'); defaulting to static"
                );
            }
        }
    }

    if cfg!(feature = "link-shared") {
        return LinkMode::Shared;
    }

    LinkMode::Static
}

fn ensure_directory(path: &Path) {
    if let Err(err) = fs::create_dir_all(path) {
        panic!("Failed to create directory {}: {err}", path.display());
    }
}

fn resolve_system_paths() -> (PathBuf, PathBuf) {
    // Preference order:
    // 1) Explicit include/lib vars
    // 2) Root prefix + include/lib
    // 3) A last-ditch attempt via common system paths (not implemented; we prefer explicit)

    let include = env::var(LIVOX_SDK2_INCLUDE_ENV).ok().map(PathBuf::from);
    let lib = env::var(LIVOX_SDK2_LIB_ENV).ok().map(PathBuf::from);

    if let (Some(include_dir), Some(lib_dir)) = (include.as_ref(), lib.as_ref()) {
        return (include_dir.clone(), lib_dir.clone());
    }

    let root = env::var(LIVOX_SDK2_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            panic!(
                "System mode requires either {LIVOX_SDK2_ROOT_ENV} or both {LIVOX_SDK2_INCLUDE_ENV} and {LIVOX_SDK2_LIB_ENV}."
            )
        });

    let include_dir = include.unwrap_or_else(|| root.join("include"));
    let lib_dir = lib.unwrap_or_else(|| root.join("lib"));

    if !include_dir.join("livox_lidar_api.h").exists() {
        println_build!(
            "Warning: {} does not contain livox_lidar_api.h",
            include_dir.display()
        );
    }

    (include_dir, lib_dir)
}

fn ensure_livox_sdk2_source() -> (PathBuf, String) {
    // 1) User-provided source path
    if let Ok(src) = env::var(LIVOX_SDK2_SOURCE_ENV) {
        let src_root = PathBuf::from(src);
        if !src_root.join("CMakeLists.txt").exists() {
            panic!(
                "{LIVOX_SDK2_SOURCE_ENV}={} does not look like a Livox-SDK2 source tree (missing CMakeLists.txt)",
                src_root.display()
            );
        }
        println_build!("Using Livox-SDK2 source from {LIVOX_SDK2_SOURCE_ENV}={}", src_root.display());
        return (src_root, "local".to_string());
    }

    // 2) Vendored clone in target/vendor
    let repo_url = env::var(LIVOX_SDK2_REPO_ENV)
        .unwrap_or_else(|_| LIVOX_SDK2_REPOSITORY_URL.to_string());
    let tag = env::var(LIVOX_SDK2_TAG_ENV).unwrap_or_else(|_| LIVOX_SDK2_TAG_DEFAULT.to_string());

    let auto = env::var(LIVOX_SDK2_AUTO_DOWNLOAD_ENV)
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    if !auto {
        panic!(
            "Vendored build requires fetching Livox-SDK2. Set {LIVOX_SDK2_AUTO_DOWNLOAD_ENV}=1 (or omit it), or use {LIVOX_SDK2_SOURCE_ENV} / feature 'system'."
        );
    }

    let safe_tag = sanitize_for_path(&tag);
    let dest = VENDOR_FOLDER_PATH.join(format!("livox-sdk2-{safe_tag}"));

    if dest.exists() && dest.join(".git").exists() {
        println_build!("Using existing Livox-SDK2 checkout at {}", dest.display());
        return (dest, tag);
    }

    println_build!("Cloning Livox-SDK2 ({tag}) into {}...", dest.display());
    clone_repository(&repo_url, &dest, Some(&tag))
        .unwrap_or_else(|err| panic!("Failed to clone Livox-SDK2 repository: {err}"));

    (dest, tag)
}

fn sanitize_for_path(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

fn clone_repository(repo_url: &str, dest_path: &Path, tag: Option<&str>) -> Result<(), String> {
    if dest_path.exists() {
        if dest_path.join(".git").exists() {
            return Ok(());
        }
        return Err(format!(
            "Destination {} exists and is not a git repository",
            dest_path.display()
        ));
    }

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;
    }

    let mut args = vec!["clone", "--recurse-submodules"];
    if let Some(tag_name) = tag {
        args.push("--branch");
        args.push(tag_name);
    }
    args.push(repo_url);
    args.push(
        dest_path
            .to_str()
            .ok_or_else(|| "Invalid destination for git clone".to_string())?,
    );

    println_build!("Running git {}", args.join(" "));

    let status = Command::new("git")
        .args(args)
        .status()
        .map_err(|e| format!("Failed to spawn git: {e}"))?;

    if !status.success() {
        return Err(format!("git clone failed with status {status}"));
    }

    Ok(())
}

fn track_livox_sdk2_sources(src_root: &Path) {
    watch_path(&src_root.join("CMakeLists.txt"));
    watch_tree(&src_root.join("sdk_core"));
    watch_tree(&src_root.join("include"));
    watch_tree(&src_root.join("3rdparty"));
}

fn apply_livox_sdk2_patches(src_root: &Path) {
    // Livox-SDK2 v1.2.x has a few headers that use fixed-width integer types
    // without including <cstdint> (or <stdint.h>), which breaks compilation on
    // some toolchains.
    //
    // We patch the vendored checkout (inside target/) in a small, idempotent way.

    add_include_cstdint_after(
        &src_root.join("sdk_core/comm/define.h"),
        "#include <stdio.h>",
    );

    add_include_cstdint_after(
        &src_root.join("sdk_core/logger_handler/file_manager.h"),
        "#include <map>",
    );

    make_samples_optional(&src_root.join("CMakeLists.txt"));
    add_windows_net_libs(&src_root.join("sdk_core/CMakeLists.txt"));
}

fn add_include_cstdint_after(path: &Path, marker_line: &str) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    if contents.contains("#include <cstdint>") {
        return;
    }

    let marker_pos = match contents.find(marker_line) {
        Some(p) => p,
        None => return,
    };

    // Insert after the end of the marker line (support both LF and CRLF).
    let after_marker = &contents[marker_pos..];
    let eol_rel = after_marker
        .find('\n')
        .map(|idx| idx + 1)
        .unwrap_or(after_marker.len());
    let insert_pos = marker_pos + eol_rel;

    let mut patched = String::with_capacity(contents.len() + 20);
    patched.push_str(&contents[..insert_pos]);
    patched.push_str("#include <cstdint>\n");
    patched.push_str(&contents[insert_pos..]);

    if let Err(e) = fs::write(path, patched) {
        println_build!("Failed to apply compatibility patch to {}: {e}", path.display());
        return;
    }

    println_build!("Applied compatibility patch: added <cstdint> to {}", path.display());
    println!("cargo:rerun-if-changed={}", path.display());
}

fn make_samples_optional(path: &Path) {
    let Ok(original) = fs::read_to_string(path) else {
        return;
    };

    let mut patched = original.clone();

    if !patched.contains("option(LIVOX_SDK2_BUILD_SAMPLES") {
        if let Some(project_pos) = patched.find("project(livox_sdk2)") {
            let after_project = &patched[project_pos..];
            let eol_rel = after_project
                .find('\n')
                .map(|idx| idx + 1)
                .unwrap_or(after_project.len());
            let insert_pos = project_pos + eol_rel;
            patched.insert_str(
                insert_pos,
                "option(LIVOX_SDK2_BUILD_SAMPLES \"Build Livox-SDK2 sample binaries\" ON)\n",
            );
        }
    }

    let old_samples_line = "add_subdirectory(samples)";
    let new_samples_guard = "if(LIVOX_SDK2_BUILD_SAMPLES)\n  add_subdirectory(samples)\nendif()";

    if patched.contains(old_samples_line) {
        patched = patched.replacen(old_samples_line, new_samples_guard, 1);
    }

    if patched == original {
        return;
    }

    if let Err(e) = fs::write(path, patched) {
        println_build!("Failed to patch sample toggles in {}: {e}", path.display());
        return;
    }

    println_build!(
        "Applied compatibility patch: made Livox-SDK2 sample build optional in {}",
        path.display()
    );
    println!("cargo:rerun-if-changed={}", path.display());
}

fn add_windows_net_libs(path: &Path) {
    let Ok(original) = fs::read_to_string(path) else {
        return;
    };

    if original.contains("target_link_libraries(${SDK_LIBRARY_SHARED} PRIVATE ws2_32 iphlpapi)") {
        return;
    }

    let marker = "install(TARGETS ${SDK_LIBRARY_STATIC} ${SDK_LIBRARY_SHARED}";
    let Some(marker_pos) = original.find(marker) else {
        return;
    };

    let link_block = "if(WIN32)\n\
target_link_libraries(${SDK_LIBRARY_STATIC} PRIVATE ws2_32 iphlpapi)\n\
target_link_libraries(${SDK_LIBRARY_SHARED} PRIVATE ws2_32 iphlpapi)\n\
endif()\n\n";

    let mut patched = String::with_capacity(original.len() + link_block.len());
    patched.push_str(&original[..marker_pos]);
    patched.push_str(link_block);
    patched.push_str(&original[marker_pos..]);

    if let Err(e) = fs::write(path, patched) {
        println_build!("Failed to patch Windows net libs in {}: {e}", path.display());
        return;
    }

    println_build!(
        "Applied compatibility patch: added ws2_32/iphlpapi linkage in {}",
        path.display()
    );
    println!("cargo:rerun-if-changed={}", path.display());
}

fn watch_path(path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn watch_tree(root: &Path) {
    if !root.exists() {
        return;
    }

    for entry in WalkDir::new(root).into_iter().filter_map(|res| res.ok()) {
        let path = entry.path();
        if path
            .components()
            .any(|component| component.as_os_str() == ".git")
        {
            continue;
        }

        if entry.file_type().is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn build_livox_sdk2(src_root: &Path, tag: &str) -> (PathBuf, PathBuf) {
    // Use a stable build/install directory under target/ to avoid rebuilding on every OUT_DIR.
    let safe_tag = sanitize_for_path(tag);
    let build_root = TARGET_DIR
        .join("vendor-build")
        .join(format!("livox-sdk2-{safe_tag}"));
    let build_dir = build_root.join("build");
    let install_dir = build_root.join("install");

    let include_dir = install_dir.join("include");
    let lib_dir = install_dir.join("lib");
    let is_windows_msvc = env::var("TARGET")
        .map(|target| target.contains("windows-msvc"))
        .unwrap_or(false);

    // Quick cache check
    let static_ok = if is_windows_msvc {
        lib_dir.join("livox_lidar_sdk_static.lib").exists()
    } else {
        lib_dir.join("liblivox_lidar_sdk_static.a").exists()
            || lib_dir.join("livox_lidar_sdk_static.lib").exists()
    };
    let shared_ok = if is_windows_msvc {
        lib_dir.join("livox_lidar_sdk_shared.lib").exists()
            || lib_dir.join("liblivox_lidar_sdk_shared.dll.a").exists()
    } else {
        lib_dir.join("liblivox_lidar_sdk_shared.so").exists()
            || lib_dir.join("livox_lidar_sdk_shared.dll").exists()
            || lib_dir.join("liblivox_lidar_sdk_shared.dll").exists()
    };
    let headers_ok = include_dir.join("livox_lidar_api.h").exists();

    if headers_ok && (static_ok || shared_ok) {
        println_build!(
            "Using cached Livox-SDK2 build at {}",
            install_dir.display()
        );
        return (include_dir, lib_dir);
    }

    ensure_directory(&build_dir);
    ensure_directory(&install_dir);

    println_build!(
        "Configuring Livox-SDK2 with CMake (build dir: {})",
        build_dir.display()
    );

    if is_windows_msvc {
        let _ = fs::remove_dir_all(&build_dir);
        ensure_directory(&build_dir);
    }

    let mut configure = Command::new("cmake");
    let mut msvc_path_entries: Vec<PathBuf> = Vec::new();
    let mut msvc_cl: Option<PathBuf> = None;
    if is_windows_msvc {
        let cl = find_msvc_cl_path()
            .unwrap_or_else(|| panic!("Unable to locate cl.exe for windows-msvc toolchain"));
        msvc_cl = Some(cl.clone());
        if let Some(cl_dir) = cl.parent() {
            msvc_path_entries.push(cl_dir.to_path_buf());
        }
        if let Some(sdk_bin_x64) = find_windows_sdk_bin_x64() {
            msvc_path_entries.push(sdk_bin_x64);
        }
        configure
            .arg("-G")
            .arg("Ninja")
            .arg(format!("-DCMAKE_C_COMPILER={}", cl.display()))
            .arg(format!("-DCMAKE_CXX_COMPILER={}", cl.display()))
            .arg("-DCMAKE_RC_COMPILER=rc")
            .arg("-DCMAKE_MT=mt")
            .arg("-DCMAKE_TRY_COMPILE_CONFIGURATION=Release")
            .arg("-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL");
        apply_path_overrides(&mut configure, &msvc_path_entries);
        if let Some(cl_path) = msvc_cl.as_deref() {
            apply_msvc_env_overrides(&mut configure, cl_path);
        }
    }
    configure
        .arg("-S")
        .arg(src_root)
        .arg("-B")
        .arg(&build_dir)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display()))
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .arg("-DLIVOX_SDK2_BUILD_SAMPLES=OFF");

    // On Windows (MinGW/GCC toolchain), Winsock symbols live in ws2_32 and
    // iphlpapi. These are not linked automatically by the Livox SDK2 CMake
    // build, causing undefined-reference errors for socket/bind/etc.
    if cfg!(target_os = "windows") && !is_windows_msvc {
        configure
            .arg("-DCMAKE_SHARED_LINKER_FLAGS=-lws2_32 -liphlpapi")
            .arg("-DCMAKE_EXE_LINKER_FLAGS=-lws2_32 -liphlpapi");
    }

    run_cmd(&mut configure, "cmake configure");

    println_build!("Building + installing Livox-SDK2...");
    let mut build = Command::new("cmake");
    build
        .arg("--build")
        .arg(&build_dir)
        .arg("--target")
        .arg("install")
        .arg("--config")
        .arg("Release");
    if is_windows_msvc {
        apply_path_overrides(&mut build, &msvc_path_entries);
        if let Some(cl_path) = msvc_cl.as_deref() {
            apply_msvc_env_overrides(&mut build, cl_path);
        }
    }
    run_cmd(&mut build, "cmake build");

    if !include_dir.join("livox_lidar_api.h").exists() {
        panic!(
            "Livox-SDK2 install did not produce expected headers in {}",
            include_dir.display()
        );
    }

    (include_dir, lib_dir)
}

fn run_cmd(cmd: &mut Command, what: &str) {
    println_build!("Running: {:?}", cmd);
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("Failed to run {what}: {e}"));
    if !status.success() {
        panic!("Command failed ({what}) with status {status}");
    }
}

fn emit_link_directives(lib_dir: &Path, link_mode: LinkMode) {
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    match link_mode {
        LinkMode::Static => {
            println!("cargo:rustc-link-lib=static=livox_lidar_sdk_static");
        }
        LinkMode::Shared => {
            println!("cargo:rustc-link-lib=dylib=livox_lidar_sdk_shared");
        }
    }

    #[cfg(target_family = "unix")]
    {
        // The upstream CMakeLists sets `-pthread` on UNIX.
        println!("cargo:rustc-link-lib=dylib=pthread");
        // When linking against C++ libraries, ensure the C++ runtime is available.
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
    }
}

fn build_bindings(include_dir: &Path) {
    let mut include_paths = vec![PROJECT_ROOT.join("src"), PROJECT_ROOT.join("src/ffi"), include_dir.to_path_buf()];
    include_paths.retain(|p| p.exists());

    let mut extra_args = vec!["-std=c++17".to_string()];

    if let Ok(target) = env::var("TARGET") {
        extra_args.push(format!("--target={target}"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Get GCC system include paths to help clang find standard headers.
        let gcc_include_output = Command::new("gcc")
            .args(["-E", "-Wp,-v", "-xc++", "/dev/null"])
            .output()
            .ok();

        if let Some(output) = gcc_include_output {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('/')
                    && (trimmed.contains("include") || trimmed.contains("gcc"))
                {
                    extra_args.push(format!("-I{}", trimmed));
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        for arg in find_msvc_include_paths() {
            extra_args.push(arg);
        }
    }

    let include_refs: Vec<&Path> = include_paths.iter().map(|p| p.as_path()).collect();
    let extra_args_refs: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();

    let builder = autocxx_build::Builder::new("src/bindings.rs", &include_refs)
        .extra_clang_args(&extra_args_refs);

    let mut cc_builder = builder
        .build()
        .expect("Unable to generate bindings for Livox-SDK2");

    for dir in &include_paths {
        cc_builder.include(dir);
    }

    cc_builder
        .file(PROJECT_ROOT.join("src/ffi/livox_wrapper.cpp"))
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-deprecated-declarations")
        .compile("carvi_livox_sdk2_binding");
}

#[cfg(not(target_os = "windows"))]
fn find_msvc_cl_path() -> Option<PathBuf> {
    unreachable!("find_msvc_cl_path is only reachable on Windows")
}

#[cfg(target_os = "windows")]
fn find_msvc_cl_path() -> Option<PathBuf> {
    let vswhere = PathBuf::from(
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    );
    if !vswhere.exists() {
        return None;
    }

    let install_path = Command::new(&vswhere)
        .args(["-latest", "-property", "installationPath"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| PathBuf::from(s.trim().to_string()))?;

    let msvc_tools = install_path.join(r"VC\Tools\MSVC");
    let latest = fs::read_dir(&msvc_tools)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .max()?;

    let cl = latest.join(r"bin\Hostx64\x64\cl.exe");
    cl.exists().then_some(cl)
}

#[cfg(not(target_os = "windows"))]
fn find_windows_sdk_bin_x64() -> Option<PathBuf> {
    unreachable!("find_windows_sdk_bin_x64 is only reachable on Windows")
}

#[cfg(target_os = "windows")]
fn find_windows_sdk_bin_x64() -> Option<PathBuf> {
    let sdk_bin_root = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let latest_version_dir = fs::read_dir(&sdk_bin_root)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                Some((name, e.path()))
            } else {
                None
            }
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, path)| path)?;
    Some(latest_version_dir.join("x64"))
}

fn apply_path_overrides(cmd: &mut Command, prepended_paths: &[PathBuf]) {
    if prepended_paths.is_empty() {
        return;
    }

    let mut merged = prepended_paths.to_vec();
    if let Some(path) = env::var_os("PATH") {
        merged.extend(env::split_paths(&path));
    }

    if let Ok(joined) = env::join_paths(merged) {
        cmd.env("PATH", joined);
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_msvc_env_overrides(_cmd: &mut Command, _cl_path: &Path) {
    unreachable!("apply_msvc_env_overrides is only reachable on Windows")
}

#[cfg(target_os = "windows")]
fn apply_msvc_env_overrides(cmd: &mut Command, cl_path: &Path) {
    let mut include_paths: Vec<PathBuf> = Vec::new();
    let mut lib_paths: Vec<PathBuf> = Vec::new();

    if let Some(msvc_root) = msvc_root_from_cl_path(cl_path) {
        include_paths.push(msvc_root.join("include"));
        lib_paths.push(msvc_root.join("lib").join("x64"));
    }

    if let Some(sdk_version) = find_windows_sdk_version() {
        let sdk_include_root = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\Include")
            .join(&sdk_version);
        for sub in ["ucrt", "um", "shared"] {
            include_paths.push(sdk_include_root.join(sub));
        }

        let sdk_lib_root =
            PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\Lib").join(&sdk_version);
        for sub in ["ucrt", "um"] {
            lib_paths.push(sdk_lib_root.join(sub).join("x64"));
        }
    }

    include_paths.retain(|path| path.exists());
    lib_paths.retain(|path| path.exists());

    if let Ok(include) = env::join_paths(include_paths) {
        cmd.env("INCLUDE", include);
    }
    if let Ok(lib) = env::join_paths(lib_paths) {
        cmd.env("LIB", &lib);
        cmd.env("LIBPATH", lib);
    }
}

#[cfg(target_os = "windows")]
fn msvc_root_from_cl_path(cl_path: &Path) -> Option<PathBuf> {
    Some(cl_path.parent()?.parent()?.parent()?.parent()?.to_path_buf())
}

#[cfg(target_os = "windows")]
fn find_windows_sdk_version() -> Option<String> {
    find_windows_sdk_bin_x64()?
        .parent()?
        .file_name()?
        .to_str()
        .map(ToOwned::to_owned)
}

#[cfg(target_os = "windows")]
fn find_msvc_include_paths() -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    if let Ok(include_env) = env::var("INCLUDE") {
        for path in include_env.split(';') {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                args.push(format!("-I{}", trimmed));
            }
        }
        if !args.is_empty() {
            return args;
        }
    }

    let vswhere = PathBuf::from(
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    );
    if !vswhere.exists() {
        println_build!(
            "vswhere.exe not found; MSVC standard headers may be unavailable for clang. \
             Consider running from a Visual Studio Developer Command Prompt."
        );
        return args;
    }

    let vs_path = Command::new(&vswhere)
        .args(["-latest", "-property", "installationPath"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| PathBuf::from(s.trim().to_string()));

    let Some(vs_path) = vs_path else {
        return args;
    };

    let msvc_tools = vs_path.join(r"VC\Tools\MSVC");
    if let Ok(entries) = fs::read_dir(&msvc_tools) {
        if let Some(latest) = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .max()
        {
            let include_dir = msvc_tools.join(&latest).join("include");
            if include_dir.exists() {
                args.push(format!("-I{}", include_dir.display()));
            }
        }
    }

    let sdk_include_root = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\Include");
    if let Ok(entries) = fs::read_dir(&sdk_include_root) {
        if let Some(sdk_version) = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .max()
        {
            for sub in &["ucrt", "um", "shared"] {
                let dir = sdk_include_root.join(&sdk_version).join(sub);
                if dir.exists() {
                    args.push(format!("-I{}", dir.display()));
                }
            }
        }
    }

    args
}

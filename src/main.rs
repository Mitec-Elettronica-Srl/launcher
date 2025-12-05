#![no_main]

use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;
use std::process;

#[unsafe(no_mangle)]
#[inline(never)]
fn main() {
    loop {
        if let Some(executable) = find_highest_version_executable() {
            exec_executable(&executable);
            // execve never returns on success, so if we reach here, it failed
            eprintln!("Failed to execute: {}", executable);
            process::exit(1);
        }
        // No executable found, loop indefinitely
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn find_highest_version_executable() -> Option<String> {
    let current_dir = std::env::current_dir().ok()?;
    let mut executables: Vec<(String, (u32, u32, u32))> = Vec::new();

    let entries = std::fs::read_dir(&current_dir).ok()?;

    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();

        // Check if it's a file and executable
        if !path.is_file() {
            continue;
        }

        let metadata = entry.metadata().ok()?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            continue; // Not executable
        }

        // Get filename
        let filename = path.file_name()?.to_str()?.to_string();

        // Check if it matches vx.y.z pattern
        if let Some(version) = parse_version(&filename) {
            executables.push((filename, version));
        }
    }

    if executables.is_empty() {
        return None;
    }

    // Sort by version (highest first) and return the first one
    executables.sort_by(|a, b| b.1.cmp(&a.1));
    Some(executables[0].0.clone())
}

fn parse_version(filename: &str) -> Option<(u32, u32, u32)> {
    // Check if filename starts with 'v' and matches pattern vx, vx.y, or vx.y.z
    if !filename.starts_with('v') {
        return None;
    }

    let version_part = &filename[1..];
    let parts: Vec<&str> = version_part.split('.').collect();

    // Accept 1, 2, or 3 parts; missing parts default to 0
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }

    let major = parts[0].parse::<u32>().ok()?;
    let minor = if parts.len() > 1 {
        parts[1].parse::<u32>().ok()?
    } else {
        0
    };
    let patch = if parts.len() > 2 {
        parts[2].parse::<u32>().ok()?
    } else {
        0
    };

    Some((major, minor, patch))
}

fn exec_executable(executable: &str) {
    let c_executable = CString::new(executable).unwrap();
    let c_args = vec![c_executable.clone()];
    let c_env: Vec<CString> = std::env::vars()
        .map(|(k, v)| CString::new(format!("{}={}", k, v)).unwrap())
        .collect();

    // Build null-terminated arrays
    let mut args: Vec<*const libc::c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
    args.push(std::ptr::null()); // null terminator

    let mut env: Vec<*const libc::c_char> = c_env.iter().map(|s| s.as_ptr()).collect();
    env.push(std::ptr::null()); // null terminator

    unsafe {
        libc::execve(
            c_executable.as_ptr(),
            args.as_ptr() as *mut *const libc::c_char,
            env.as_ptr() as *mut *const libc::c_char,
        );
    }
}

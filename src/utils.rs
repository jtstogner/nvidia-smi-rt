use std::fs;
use std::path::Path;

/// Formats byte counts into human-readable strings (B, KiB, MiB, GiB, TiB).
pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    const TIB: u64 = GIB * 1024;

    if bytes >= TIB {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats megahertz clock speed.
pub fn format_mhz(mhz: u32) -> String {
    if mhz >= 1000 {
        format!("{:.2} GHz", mhz as f64 / 1000.0)
    } else {
        format!("{} MHz", mhz)
    }
}

/// Formats watts.
pub fn format_watts(watts: f64) -> String {
    format!("{:.1} W", watts)
}

/// Formats temperature with unit.
pub fn format_temp(temp_c: u32) -> String {
    format!("{}°C", temp_c)
}

/// Reads the executable name for a given PID from /proc/{pid}/comm or cmdline on Linux.
pub fn get_process_info_from_proc(pid: u32) -> (String, String) {
    let comm_path = format!("/proc/{}/comm", pid);
    let cmdline_path = format!("/proc/{}/cmdline", pid);

    let name = fs::read_to_string(&comm_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| {
            // Fallback to reading first token of cmdline
            if let Ok(cmd) = fs::read_to_string(&cmdline_path) {
                let first = cmd.split('\0').next().unwrap_or("").trim();
                if let Some(base) = Path::new(first).file_name() {
                    return base.to_string_lossy().to_string();
                }
            }
            format!("PID:{}", pid)
        });

    let cmdline = fs::read(&cmdline_path)
        .map(|bytes| {
            let mut parts = Vec::new();
            for part in bytes.split(|&b| b == 0) {
                if !part.is_empty() {
                    parts.push(String::from_utf8_lossy(part).to_string());
                }
            }
            if parts.is_empty() {
                name.clone()
            } else {
                parts.join(" ")
            }
        })
        .unwrap_or_else(|_| name.clone());

    (name, cmdline)
}

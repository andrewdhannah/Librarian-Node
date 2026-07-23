use super::HardwareDetector;
use std::process::Command;

pub struct WindowsHardwareDetector;

impl HardwareDetector for WindowsHardwareDetector {
    fn detect_gpu_vendor(&self) -> Option<String> {
        run_wmic("PATH Win32_VideoController GET Name").and_then(|output| {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() >= 2 {
                let name = lines[1].trim().to_string();
                if name.is_empty() { None } else {
                    if name.to_lowercase().contains("nvidia") {
                        Some("NVIDIA".to_string())
                    } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
                        Some("AMD".to_string())
                    } else if name.to_lowercase().contains("intel") {
                        Some("Intel".to_string())
                    } else {
                        Some("Unknown".to_string())
                    }
                }
            } else {
                None
            }
        })
    }

    fn detect_gpu_model(&self) -> Option<String> {
        run_wmic("PATH Win32_VideoController GET Name").and_then(|output| {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() >= 2 {
                let name = lines[1].trim().to_string();
                if name.is_empty() { None } else { Some(name) }
            } else {
                None
            }
        })
    }

    fn detect_gpu_vram_mb(&self) -> Option<u64> {
        run_wmic("PATH Win32_VideoController GET AdapterRAM").and_then(|output| {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() >= 2 {
                lines[1].trim().parse::<u64>().ok().map(|bytes| bytes / (1024 * 1024))
            } else {
                None
            }
        })
    }

    fn detect_total_ram_mb(&self) -> Option<u64> {
        run_wmic("OS GET TotalVisibleMemorySize").and_then(|output| {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() >= 2 {
                lines[1].trim().parse::<u64>().ok().map(|kb| kb / 1024)
            } else {
                None
            }
        })
    }

    fn detect_cpu_model(&self) -> Option<String> {
        run_wmic("PATH Win32_Processor GET Name").and_then(|output| {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() >= 2 {
                let name = lines[1].trim().to_string();
                if name.is_empty() { None } else { Some(name) }
            } else {
                None
            }
        })
    }

    fn detect_cpu_cores(&self) -> Option<u32> {
        run_wmic("PATH Win32_Processor GET NumberOfCores").and_then(|output| {
            let lines: Vec<&str> = output.lines().collect();
            if lines.len() >= 2 {
                lines[1].trim().parse::<u32>().ok()
            } else {
                None
            }
        })
    }

    fn platform_name(&self) -> String {
        "windows".to_string()
    }
}

fn run_wmic(query: &str) -> Option<String> {
    let parts: Vec<&str> = query.splitn(2, ' ').collect();
    let (subcmd, args) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        (query, "")
    };

    Command::new("wmic")
        .arg(subcmd)
        .arg(args)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                if stdout.trim().is_empty() { None } else { Some(stdout) }
            } else {
                None
            }
        })
}

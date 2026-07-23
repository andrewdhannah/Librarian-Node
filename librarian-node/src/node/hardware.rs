use crate::db::RuntimeDatabase;
use crate::platform::{create_detector, HardwareDetector};
use librarian_contracts::node::HardwareProfile;

pub fn detect_hardware(db: &RuntimeDatabase) -> HardwareProfile {
    let detector = create_detector();

    let (gpu_vendor, gpu_model, gpu_vram) = db.list_hardware_profiles().ok().and_then(|profiles| {
        profiles.first().map(|hw| {
            (
                hw.vulkan_device
                    .as_ref()
                    .map(|d| {
                        if d.to_lowercase().contains("nvidia") {
                            "NVIDIA"
                        } else if d.to_lowercase().contains("amd")
                            || d.to_lowercase().contains("radeon")
                        {
                            "AMD"
                        } else if d.to_lowercase().contains("intel") {
                            "Intel"
                        } else {
                            "Unknown"
                        }
                        .to_string()
                    })
                    .or_else(|| detector.detect_gpu_vendor()),
                hw.device_name.clone().or_else(|| detector.detect_gpu_model()),
                hw.total_vram_mb.map(|v| v as u64).or_else(|| detector.detect_gpu_vram_mb()),
            )
        })
    })
    .unwrap_or_else(|| {
        (
            detector.detect_gpu_vendor(),
            detector.detect_gpu_model(),
            detector.detect_gpu_vram_mb(),
        )
    });

    let cpu_model = detector.detect_cpu_model();
    let cpu_cores = detector.detect_cpu_cores();
    let total_ram = detector.detect_total_ram_mb();

    HardwareProfile {
        cpu_model,
        cpu_cores,
        total_ram_mb: total_ram,
        gpu_vendor,
        gpu_model,
        gpu_vram_mb: gpu_vram,
        os_platform: detector.platform_name(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_db() -> RuntimeDatabase {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = RuntimeDatabase::open(path).unwrap();
        db.migrate().unwrap();
        Box::leak(Box::new(dir));
        db
    }

    #[test]
    fn test_detect_hardware_returns_expected_fields() {
        let db = test_db();
        let hw = detect_hardware(&db);
        // These are best-effort; just check they exist
        assert!(!hw.os_platform.is_empty());
        // Optional fields may be None on systems without WMI, that's fine
    }
}

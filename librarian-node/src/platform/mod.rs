pub mod windows;

pub trait HardwareDetector: Send + Sync {
    fn detect_gpu_vendor(&self) -> Option<String>;
    fn detect_gpu_model(&self) -> Option<String>;
    fn detect_gpu_vram_mb(&self) -> Option<u64>;
    fn detect_total_ram_mb(&self) -> Option<u64>;
    fn detect_cpu_model(&self) -> Option<String>;
    fn detect_cpu_cores(&self) -> Option<u32>;
    fn platform_name(&self) -> String;
}

#[cfg(target_os = "windows")]
pub fn create_detector() -> windows::WindowsHardwareDetector {
    windows::WindowsHardwareDetector
}

#[cfg(not(target_os = "windows"))]
pub fn create_detector() -> NullHardwareDetector {
    NullHardwareDetector
}

#[cfg(not(target_os = "windows"))]
pub struct NullHardwareDetector;

#[cfg(not(target_os = "windows"))]
impl HardwareDetector for NullHardwareDetector {
    fn detect_gpu_vendor(&self) -> Option<String> { None }
    fn detect_gpu_model(&self) -> Option<String> { None }
    fn detect_gpu_vram_mb(&self) -> Option<u64> { None }
    fn detect_total_ram_mb(&self) -> Option<u64> { None }
    fn detect_cpu_model(&self) -> Option<String> { None }
    fn detect_cpu_cores(&self) -> Option<u32> { None }
    fn platform_name(&self) -> String { std::env::consts::OS.to_string() }
}

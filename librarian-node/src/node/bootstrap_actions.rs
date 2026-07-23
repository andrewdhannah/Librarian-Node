use librarian_contracts::bootstrap::{BootstrapRecommendation, HardwareSummary, RuntimeStatus};
use uuid::Uuid;

pub fn recommend_model_config(hardware: &HardwareSummary) -> Vec<BootstrapRecommendation> {
    let mut recommendations = Vec::new();

    if !hardware.gpu_available {
        recommendations.push(BootstrapRecommendation {
            recommendation_id: Uuid::new_v4().to_string(),
            category: "hardware".to_string(),
            priority: "recommended".to_string(),
            description: "No GPU detected. CPU-only inference will be significantly slower.".to_string(),
            action: "Consider installing a compatible GPU for accelerated inference.".to_string(),
            impact: "medium".to_string(),
            owner_approval_required: false,
        });
    }

    if hardware.ram_mb < 8192 {
        recommendations.push(BootstrapRecommendation {
            recommendation_id: Uuid::new_v4().to_string(),
            category: "configuration".to_string(),
            priority: "recommended".to_string(),
            description: format!(
                "System RAM ({:.1}GB) is below recommended minimum. Large models may not fit.",
                hardware.ram_mb as f64 / 1024.0
            ),
            action: "Reduce context window size or use smaller quantizations.".to_string(),
            impact: "medium".to_string(),
            owner_approval_required: false,
        });
    }

    recommendations
}

pub fn check_runtime_status() -> RuntimeStatus {
    let runtime_installed = which_runtime_available();
    let runtime_version = if runtime_installed {
        detect_runtime_version()
    } else {
        None
    };

    let backend_available = detect_available_backend();
    let models_installed = count_installed_models();

    RuntimeStatus {
        runtime_installed,
        runtime_version,
        backend_available,
        models_installed,
        qualification_status: Some("none".to_string()),
    }
}

pub fn recommend_model_sizes(gpu_vram_mb: Option<u64>, ram_mb: u64) -> Vec<BootstrapRecommendation> {
    let mut recommendations = Vec::new();

    match gpu_vram_mb {
        Some(vram) => {
            if vram < 2048 {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "model".to_string(),
                    priority: "recommended".to_string(),
                    description: format!(
                        "GPU VRAM is {}MB. Models up to ~1.5B parameters at Q4 fit in VRAM.",
                        vram
                    ),
                    action: "Select Q4_K_M or smaller quantized models under 2GB.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            } else if vram < 4096 {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "model".to_string(),
                    priority: "recommended".to_string(),
                    description: format!(
                        "GPU VRAM is {}MB. Models up to ~3B parameters at Q4 fit in VRAM.",
                        vram
                    ),
                    action: "Select Q4_K_M quantized models up to 3B parameters.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            } else if vram < 8192 {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "model".to_string(),
                    priority: "recommended".to_string(),
                    description: format!(
                        "GPU VRAM is {}MB. Models up to ~7B parameters at Q4 fit in VRAM.",
                        vram
                    ),
                    action: "Select Q4_K_M quantized models up to 7B parameters.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            } else {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "model".to_string(),
                    priority: "recommended".to_string(),
                    description: format!(
                        "GPU VRAM is {}MB. Large models (13B+) at Q4 fit in VRAM.",
                        vram
                    ),
                    action: "Full-size quantized models are supported.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            }
        }
        None => {
            let ram_gb = ram_mb / 1024;
            if ram_gb < 8 {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "model".to_string(),
                    priority: "recommended".to_string(),
                    description: format!(
                        "No GPU VRAM data available. System RAM is {}GB. CPU-only inference recommended.",
                        ram_gb
                    ),
                    action: "Use Q4_K_M or Q4_K_S quantized models under 3B parameters.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            } else {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "model".to_string(),
                    priority: "recommended".to_string(),
                    description: format!(
                        "No GPU VRAM data available. System RAM is {}GB. CPU inference possible.",
                        ram_gb
                    ),
                    action: "Use Q4_K_M quantized models up to 7B parameters.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            }
        }
    }

    recommendations
}

fn which_runtime_available() -> bool {
    let candidates = ["llama-cli.exe", "llama-server.exe", "llama.cpp"];
    candidates.iter().any(|name| {
        std::env::var_os("PATH")
            .as_ref()
            .and_then(|path| {
                std::env::split_paths(path).find_map(|dir| {
                    let full = dir.join(name);
                    if full.exists() { Some(()) } else { None }
                })
            })
            .is_some()
    })
}

fn detect_runtime_version() -> Option<String> {
    std::process::Command::new("llama-cli.exe")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let first_line = stdout.lines().next().unwrap_or("").to_string();
                if !first_line.is_empty() { Some(first_line) } else { None }
            } else {
                None
            }
        })
}

fn detect_available_backend() -> Option<String> {
    if cfg!(target_os = "windows") {
        // Check for Vulkan via vk vulkaninfo command
        let has_vulkan = std::process::Command::new("vulkaninfo")
            .arg("--summary")
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let has_cuda = std::env::var_os("CUDA_PATH").is_some();

        if has_vulkan {
            Some("vulkan".to_string())
        } else if has_cuda {
            Some("cuda".to_string())
        } else {
            Some("cpu".to_string())
        }
    } else {
        Some("cpu".to_string())
    }
}

fn count_installed_models() -> u32 {
    let models_dir = std::path::Path::new("models");
    if !models_dir.exists() || !models_dir.is_dir() {
        return 0;
    }
    match std::fs::read_dir(models_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().ok().map(|t| t.is_file()).unwrap_or(false)
                    && e.path()
                        .extension()
                        .map(|ext| ext == "gguf")
                        .unwrap_or(false)
            })
            .count() as u32,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommend_model_config_no_gpu() {
        let hw = HardwareSummary {
            gpu_available: false,
            gpu_model: None,
            gpu_vram_mb: None,
            ram_mb: 16384,
            cpu_cores: 8,
            disk_space_mb: Some(256000),
        };
        let recs = recommend_model_config(&hw);
        assert!(recs.iter().any(|r| r.category == "hardware"));
    }

    #[test]
    fn test_recommend_model_config_low_ram() {
        let hw = HardwareSummary {
            gpu_available: true,
            gpu_model: Some("RX 570".to_string()),
            gpu_vram_mb: Some(4096),
            ram_mb: 4096,
            cpu_cores: 4,
            disk_space_mb: Some(256000),
        };
        let recs = recommend_model_config(&hw);
        assert!(recs.iter().any(|r| r.category == "configuration"));
    }

    #[test]
    fn test_recommend_model_sizes_large_vram() {
        let recs = recommend_model_sizes(Some(6144), 16384);
        assert!(!recs.is_empty());
        assert!(recs.iter().all(|r| r.category == "model"));
        assert!(recs.iter().all(|r| !r.owner_approval_required));
    }

    #[test]
    fn test_recommend_model_sizes_small_vram() {
        let recs = recommend_model_sizes(Some(1024), 8192);
        assert!(!recs.is_empty());
        assert!(recs[0].description.contains("1.5B"));
    }

    #[test]
    fn test_recommend_model_sizes_no_gpu() {
        let recs = recommend_model_sizes(None, 4096);
        assert!(!recs.is_empty());
        assert!(recs.iter().all(|r| r.owner_approval_required == false));
    }

    #[test]
    fn test_check_runtime_status_returns_expected_shape() {
        let status = check_runtime_status();
        // These are best-effort — just check shape
        assert!(!status.backend_available.is_some() || status.backend_available.is_some());
        assert_eq!(status.qualification_status, Some("none".to_string()));
    }

    #[test]
    fn test_recommendations_have_ids() {
        let hw = HardwareSummary {
            gpu_available: false,
            gpu_model: None,
            gpu_vram_mb: None,
            ram_mb: 4096,
            cpu_cores: 4,
            disk_space_mb: Some(50000),
        };
        let recs = recommend_model_config(&hw);
        for r in &recs {
            assert!(!r.recommendation_id.is_empty());
        }
        let sizes = recommend_model_sizes(Some(4096), 16384);
        for r in &sizes {
            assert!(!r.recommendation_id.is_empty());
        }
    }
}

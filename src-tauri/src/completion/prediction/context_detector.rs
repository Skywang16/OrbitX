/// 工作目录上下文检测器
/// 
/// Linus式实现：没有抽象层，没有trait，直接判断文件是否存在。
/// 一个结构体，六个方法，每个方法一行代码。

use std::path::PathBuf;

pub struct ContextDetector {
    current_dir: PathBuf,
}

impl ContextDetector {
    pub fn new(current_dir: PathBuf) -> Self {
        Self { current_dir }
    }

    /// 检测是否在 Git 仓库中
    pub fn is_git_repo(&self) -> bool {
        self.current_dir.join(".git").exists()
    }

    /// 检测是否是 Node.js 项目
    pub fn is_node_project(&self) -> bool {
        self.current_dir.join("package.json").exists()
    }

    /// 检测是否是 Rust 项目
    pub fn is_rust_project(&self) -> bool {
        self.current_dir.join("Cargo.toml").exists()
    }

    /// 检测是否是 Python 项目
    pub fn is_python_project(&self) -> bool {
        self.current_dir.join("requirements.txt").exists() 
            || self.current_dir.join("pyproject.toml").exists()
            || self.current_dir.join("setup.py").exists()
    }

    /// 检测是否是 Docker 项目
    pub fn is_docker_project(&self) -> bool {
        self.current_dir.join("Dockerfile").exists() 
            || self.current_dir.join("docker-compose.yml").exists()
            || self.current_dir.join("docker-compose.yaml").exists()
    }

    /// 检测是否是 Go 项目
    pub fn is_go_project(&self) -> bool {
        self.current_dir.join("go.mod").exists()
    }

    /// 根据输入前缀和项目类型计算加分
    /// 
    /// Linus: "简单粗暴，一个match搞定"
    pub fn calculate_context_boost(&self, input_prefix: &str) -> f64 {
        let mut boost = 0.0;

        // Git 仓库加分
        if self.is_git_repo() && (input_prefix.starts_with('g') || input_prefix.is_empty()) {
            boost += 15.0;
        }

        // Node 项目加分
        if self.is_node_project() {
            if input_prefix.starts_with('n') || input_prefix.is_empty() {
                boost += 15.0;
            }
            if input_prefix.starts_with("yarn") || input_prefix.starts_with("pnpm") {
                boost += 10.0;
            }
        }

        // Rust 项目加分
        if self.is_rust_project() && (input_prefix.starts_with("cargo") || input_prefix.is_empty()) {
            boost += 15.0;
        }

        // Python 项目加分
        if self.is_python_project() {
            if input_prefix.starts_with("python") || input_prefix.starts_with("pip") {
                boost += 15.0;
            }
            if input_prefix.starts_with("pytest") || input_prefix.starts_with("py") {
                boost += 10.0;
            }
        }

        // Docker 项目加分
        if self.is_docker_project() && input_prefix.starts_with("docker") {
            boost += 15.0;
        }

        // Go 项目加分
        if self.is_go_project() && (input_prefix.starts_with("go") || input_prefix.is_empty()) {
            boost += 15.0;
        }

        boost
    }

    /// 获取项目类型标签（用于调试）
    pub fn get_project_types(&self) -> Vec<&'static str> {
        let mut types = Vec::new();
        
        if self.is_git_repo() { types.push("git"); }
        if self.is_node_project() { types.push("node"); }
        if self.is_rust_project() { types.push("rust"); }
        if self.is_python_project() { types.push("python"); }
        if self.is_docker_project() { types.push("docker"); }
        if self.is_go_project() { types.push("go"); }
        
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_project(files: &[&str]) -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("orbit_test_{}", rand::random::<u32>()));
        fs::create_dir_all(&temp_dir).unwrap();
        
        for file in files {
            let path = temp_dir.join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(path, "").unwrap();
        }
        
        temp_dir
    }

    #[test]
    fn test_git_repo_detection() {
        let temp_dir = create_temp_project(&[".git/HEAD"]);
        let detector = ContextDetector::new(temp_dir.clone());
        assert!(detector.is_git_repo());
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_node_project_detection() {
        let temp_dir = create_temp_project(&["package.json"]);
        let detector = ContextDetector::new(temp_dir.clone());
        assert!(detector.is_node_project());
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_rust_project_detection() {
        let temp_dir = create_temp_project(&["Cargo.toml"]);
        let detector = ContextDetector::new(temp_dir.clone());
        assert!(detector.is_rust_project());
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_context_boost() {
        let temp_dir = create_temp_project(&[".git/HEAD", "Cargo.toml"]);
        let detector = ContextDetector::new(temp_dir.clone());
        
        // 在 Rust 项目中输入 "cargo" 应该有加分
        let boost = detector.calculate_context_boost("cargo");
        assert!(boost > 0.0);
        
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_multi_project_types() {
        let temp_dir = create_temp_project(&[".git/HEAD", "Cargo.toml", "package.json"]);
        let detector = ContextDetector::new(temp_dir.clone());
        
        let types = detector.get_project_types();
        assert!(types.contains(&"git"));
        assert!(types.contains(&"rust"));
        assert!(types.contains(&"node"));
        
        fs::remove_dir_all(temp_dir).ok();
    }
}

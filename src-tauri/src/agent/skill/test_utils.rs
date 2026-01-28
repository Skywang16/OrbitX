//! 测试工具模块 - 提供通用的测试辅助函数

#![cfg(test)]

use std::fs as std_fs;
use std::path::Path;

/// 创建标准格式的测试技能
pub fn create_test_skill(dir: &Path, name: &str) -> std::io::Result<()> {
    std_fs::create_dir_all(dir)?;

    let skill_md = format!(
        r#"---
name: {}
description: Test skill for {}
license: MIT
---

# {}

Test content.
"#,
        name, name, name
    );

    std_fs::write(dir.join("SKILL.md"), skill_md)?;
    Ok(())
}

/// 创建包含多个技能的测试工作空间
pub fn create_test_workspace(dir: &Path) -> std::io::Result<()> {
    let skills_dir = dir.join(".claude").join("skills");
    std_fs::create_dir_all(&skills_dir)?;

    // 创建测试技能
    for (name, desc) in &[
        ("pdf-processing", "Process PDF files"),
        ("code-review", "Review code quality"),
        ("data-analysis", "Analyze data"),
    ] {
        let skill_dir = skills_dir.join(name);
        std_fs::create_dir_all(&skill_dir)?;

        let skill_md = format!(
            r#"---
name: {}
description: {}
---

# {}

Instructions for {}.
"#,
            name, desc, name, name
        );

        std_fs::write(skill_dir.join("SKILL.md"), skill_md)?;
    }

    Ok(())
}

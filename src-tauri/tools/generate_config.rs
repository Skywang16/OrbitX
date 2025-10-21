//! 配置文件生成工具

use terminal_lib::config::defaults::create_default_config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_config = create_default_config();

    // 序列化为TOML
    let toml_content = toml::to_string_pretty(&default_config)?;

    // 输出到控制台
    println!("=== 默认配置文件内容 ===");
    println!("{}", toml_content);

    // 保存到文件
    let config_dir = dirs::config_dir().ok_or("无法获取配置目录")?.join("OrbitX");

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)?;
    }

    let config_path = config_dir.join("config.toml");
    tokio::fs::write(&config_path, toml_content).await?;

    println!("\n=== 配置文件已保存到: {:?} ===", config_path);

    Ok(())
}

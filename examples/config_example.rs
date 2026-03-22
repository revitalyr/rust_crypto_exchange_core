//! Example of using configuration in crypto exchange clients
//! 
//! This example demonstrates how to load and use configuration
//! from different sources (file, environment, defaults).

use crypto_exchange_common::config::AppConfig;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🔧 Configuration Example");
    println!("====================");

    // Load configuration (tries environment, then files, then defaults)
    let config = AppConfig::load()?;
    
    println!("✅ Configuration loaded successfully!");
    println!();
    
    // Display server configuration
    println!("🌐 Server Configuration:");
    println!("  URL: {}", config.server.url);
    println!("  Timeout: {}s", config.server.timeout);
    println!("  Reconnect interval: {}s", config.server.reconnect_interval);
    println!("  Max reconnect attempts: {}", config.server.max_reconnect_attempts);
    println!();
    
    // Display client configuration
    println!("💻 Client Configuration:");
    println!("  Name: {}", config.client.name);
    println!("  Version: {}", config.client.version);
    println!("  Auto-reconnect: {}", config.client.auto_reconnect);
    println!();
    
    // Display trading configuration
    println!("💰 Trading Configuration:");
    println!("  Default pair: {}", config.trading.default_pair);
    println!("  Max order size: {}", config.trading.max_order_size);
    println!("  Order timeout: {}s", config.trading.order_timeout);
    println!();
    
    // Display UI configuration
    println!("🎨 UI Configuration:");
    println!("  Theme: {}", config.ui.theme);
    println!("  Chart update interval: {}ms", config.ui.chart_update_interval);
    println!("  Max chart points: {}", config.ui.max_chart_points);
    println!();
    
    // Display logging configuration
    println!("📝 Logging Configuration:");
    println!("  Level: {}", config.logging.level);
    match &config.logging.file {
        Some(file) => println!("  File: {}", file),
        None => println!("  File: console only"),
    }
    println!();
    
    // Example of saving configuration
    println!("💾 Saving configuration example...");
    if let Err(e) = config.save_to_file("example_output.toml") {
        eprintln!("❌ Failed to save configuration: {}", e);
    } else {
        println!("✅ Configuration saved to example_output.toml");
    }
    
    Ok(())
}

//! Configuration management for crypto exchange clients
//! 
//! Provides configuration loading and validation functionality
//! for different environments (development, test, production).

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, Context};
use std::fs;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// Reconnect interval in seconds
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval: u64,
    /// Maximum reconnection attempts
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:8080/ws".to_string(),
            timeout: 30,
            reconnect_interval: 5,
            max_reconnect_attempts: 10,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Client name
    #[serde(default = "default_client_name")]
    pub name: String,
    /// Client version
    #[serde(default = "default_version")]
    pub version: String,
    /// Auto-reconnect enabled
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "Desktop Trading Client".to_string(),
            version: "1.0.0".to_string(),
            auto_reconnect: true,
        }
    }
}

/// Trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Default trading pair
    #[serde(default = "default_pair")]
    pub default_pair: String,
    /// Maximum order size
    #[serde(default = "default_max_order_size")]
    pub max_order_size: u64,
    /// Default order timeout in seconds
    #[serde(default = "default_order_timeout")]
    pub order_timeout: u64,
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            default_pair: "BTC/USDT".to_string(),
            max_order_size: 1000000,
            order_timeout: 60,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Theme (light, dark, auto)
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Chart update interval in milliseconds
    #[serde(default = "default_chart_update_interval")]
    pub chart_update_interval: u64,
    /// Maximum chart points
    #[serde(default = "default_max_chart_points")]
    pub max_chart_points: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "auto".to_string(),
            chart_update_interval: 1000,
            max_chart_points: 100,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log file path (optional)
    pub file: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
        }
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,
    /// Client configuration
    #[serde(default)]
    pub client: ClientConfig,
    /// Trading configuration
    #[serde(default)]
    pub trading: TradingConfig,
    /// UI configuration
    #[serde(default)]
    pub ui: UiConfig,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            client: ClientConfig::default(),
            trading: TradingConfig::default(),
            ui: UiConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        let config: Self = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
        
        Ok(config)
    }
    
    /// Load configuration from environment or default
    pub fn load() -> Result<Self> {
        // Try to load from environment variable
        if let Ok(config_path) = std::env::var("CRYPTO_EXCHANGE_CONFIG") {
            Self::load_from_file(config_path)
        } else {
            // Try default locations
            let default_paths = [
                "config/production.toml",
                "config/default.toml",
                "config/test.toml",
            ];
            
            for path in default_paths {
                if Path::new(path).exists() {
                    return Self::load_from_file(path);
                }
            }
            
            // Return default configuration if no file found
            Ok(Self::default())
        }
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration")?;
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path.as_ref()))?;
        
        Ok(())
    }
}

// Default value functions
fn default_client_name() -> String {
    "Desktop Trading Client".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_auto_reconnect() -> bool {
    true
}

fn default_pair() -> String {
    "BTC/USDT".to_string()
}

fn default_max_order_size() -> u64 {
    1000000
}

fn default_order_timeout() -> u64 {
    60
}

fn default_theme() -> String {
    "auto".to_string()
}

fn default_chart_update_interval() -> u64 {
    1000
}

fn default_max_chart_points() -> usize {
    100
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_reconnect_interval() -> u64 {
    5
}

fn default_max_reconnect_attempts() -> u32 {
    10
}

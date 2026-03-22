"""Configuration management for Python trading client"""

import os
import toml
from typing import Optional, Dict, Any
from dataclasses import dataclass, field


@dataclass
class ServerConfig:
    """Server configuration"""
    url: str = "ws://127.0.0.1:8080/ws"
    timeout: int = 30
    reconnect_interval: int = 5
    max_reconnect_attempts: int = 10


@dataclass
class ClientConfig:
    """Client configuration"""
    name: str = "Python Trading Client"
    version: str = "1.0.0"
    auto_reconnect: bool = True


@dataclass
class TradingConfig:
    """Trading configuration"""
    default_pair: str = "BTC/USDT"
    max_order_size: int = 1000000
    order_timeout: int = 60


@dataclass
class LoggingConfig:
    """Logging configuration"""
    level: str = "info"
    file: Optional[str] = None


@dataclass
class AppConfig:
    """Main application configuration"""
    server: ServerConfig = field(default_factory=ServerConfig)
    client: ClientConfig = field(default_factory=ClientConfig)
    trading: TradingConfig = field(default_factory=TradingConfig)
    logging: LoggingConfig = field(default_factory=LoggingConfig)

    @classmethod
    def load_from_file(cls, config_path: str) -> 'AppConfig':
        """Load configuration from TOML file"""
        try:
            with open(config_path, 'r') as f:
                data = toml.load(f)
                return cls.from_dict(data)
        except FileNotFoundError:
            print(f"Config file not found: {config_path}. Using defaults.")
            return cls()
        except Exception as e:
            print(f"Error loading config: {e}. Using defaults.")
            return cls()

    @classmethod
    def load(cls) -> 'AppConfig':
        """Load configuration from environment or default locations"""
        # Try environment variable first
        config_path = os.getenv('CRYPTO_EXCHANGE_CONFIG')
        
        if config_path:
            return cls.load_from_file(config_path)
        
        # Try default locations
        default_paths = [
            'config/production.toml',
            'config/default.toml',
            'config/test.toml',
        ]
        
        for path in default_paths:
            if os.path.exists(path):
                return cls.load_from_file(path)
        
        # Return default configuration
        print("No config file found. Using defaults.")
        return cls()

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'AppConfig':
        """Create configuration from dictionary"""
        server_data = data.get('server', {})
        client_data = data.get('client', {})
        trading_data = data.get('trading', {})
        logging_data = data.get('logging', {})
        
        return cls(
            server=ServerConfig(**server_data),
            client=ClientConfig(**client_data),
            trading=TradingConfig(**trading_data),
            logging=LoggingConfig(**logging_data)
        )

    def save_to_file(self, config_path: str) -> None:
        """Save configuration to TOML file"""
        data = {
            'server': self.server.__dict__,
            'client': self.client.__dict__,
            'trading': self.trading.__dict__,
            'logging': self.logging.__dict__
        }
        
        try:
            os.makedirs(os.path.dirname(config_path), exist_ok=True)
            with open(config_path, 'w') as f:
                toml.dump(data, f)
            print(f"Configuration saved to: {config_path}")
        except Exception as e:
            print(f"Error saving config: {e}")

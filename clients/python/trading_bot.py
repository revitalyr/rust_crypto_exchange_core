#!/usr/bin/env python3
"""
Advanced Trading Bot with Multiple Strategies
Implements various trading algorithms and risk management
"""

import asyncio
import json
import websockets
import random
import time
import numpy as np
import pandas as pd
from typing import Dict, List, Optional, Tuple, Callable
from dataclasses import dataclass, field
from enum import Enum
import logging
from collections import deque
import statistics

from trading_client import ExchangeClient, Platform, OrderSide, OrderType, Balance, Trade

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class Signal(Enum):
    BUY = "buy"
    SELL = "sell"
    HOLD = "hold"

@dataclass
class Position:
    symbol: str
    side: str
    quantity: float
    entry_price: float
    current_price: float
    unrealized_pnl: float = 0.0
    realized_pnl: float = 0.0
    entry_time: float = field(default_factory=time.time)

@dataclass
class MarketData:
    symbol: str
    bid: float
    ask: float
    bid_size: float
    ask_size: float
    mid_price: float
    spread: float
    timestamp: float = field(default_factory=time.time)

class TechnicalIndicators:
    """Technical analysis indicators"""
    
    @staticmethod
    def sma(prices: List[float], period: int) -> float:
        """Simple Moving Average"""
        if len(prices) < period:
            return prices[-1] if prices else 0.0
        return sum(prices[-period:]) / period
    
    @staticmethod
    def ema(prices: List[float], period: int) -> float:
        """Exponential Moving Average"""
        if len(prices) < period:
            return prices[-1] if prices else 0.0
        
        multiplier = 2 / (period + 1)
        ema = prices[0]
        
        for price in prices[1:]:
            ema = (price * multiplier) + (ema * (1 - multiplier))
        
        return ema
    
    @staticmethod
    def rsi(prices: List[float], period: int = 14) -> float:
        """Relative Strength Index"""
        if len(prices) < period + 1:
            return 50.0
        
        deltas = [prices[i] - prices[i-1] for i in range(1, len(prices))]
        gains = [d if d > 0 else 0 for d in deltas]
        losses = [-d if d < 0 else 0 for d in deltas]
        
        avg_gain = statistics.mean(gains[-period:])
        avg_loss = statistics.mean(losses[-period:])
        
        if avg_loss == 0:
            return 100.0
        
        rs = avg_gain / avg_loss
        rsi = 100 - (100 / (1 + rs))
        
        return rsi
    
    @staticmethod
    def bollinger_bands(prices: List[float], period: int = 20, std_dev: float = 2.0) -> Tuple[float, float, float]:
        """Bollinger Bands"""
        if len(prices) < period:
            price = prices[-1] if prices else 0.0
            return price, price, price
        
        sma = TechnicalIndicators.sma(prices, period)
        std = statistics.stdev(prices[-period:])
        
        upper_band = sma + (std_dev * std)
        lower_band = sma - (std_dev * std)
        
        return lower_band, sma, upper_band
    
    @staticmethod
    def macd(prices: List[float], fast: int = 12, slow: int = 26, signal: int = 9) -> Tuple[float, float, float]:
        """MACD (Moving Average Convergence Divergence)"""
        if len(prices) < slow + signal:
            return 0.0, 0.0, 0.0
        
        fast_ema = TechnicalIndicators.ema(prices, fast)
        slow_ema = TechnicalIndicators.ema(prices, slow)
        macd_line = fast_ema - slow_ema
        
        # For signal line, we'd need MACD history, simplified here
        signal_line = macd_line * 0.9  # Simplified
        histogram = macd_line - signal_line
        
        return macd_line, signal_line, histogram

class RiskManager:
    """Risk management and position sizing"""
    
    def __init__(self, max_position_size: float = 0.1, max_risk_per_trade: float = 0.02):
        self.max_position_size = max_position_size  # 10% of portfolio
        self.max_risk_per_trade = max_risk_per_trade  # 2% risk per trade
        self.stop_loss_pct = 0.02  # 2% stop loss
        self.take_profit_pct = 0.04  # 4% take profit
    
    def calculate_position_size(self, balance: float, price: float, volatility: float) -> float:
        """Calculate optimal position size based on risk"""
        if price <= 0:
            return 0.0
        
        risk_amount = balance * self.max_risk_per_trade
        stop_loss_amount = price * self.stop_loss_pct
        
        # Adjust for volatility
        volatility_adjustment = min(1.0, 0.02 / max(volatility, 0.001))
        
        position_size = (risk_amount / stop_loss_amount) * volatility_adjustment
        max_size = balance * self.max_position_size
        
        return min(position_size, max_size)
    
    def should_stop_loss(self, position: Position, current_price: float) -> bool:
        """Check if position should be stopped out"""
        if position.side == "buy":
            loss_pct = (position.entry_price - current_price) / position.entry_price
        else:
            loss_pct = (current_price - position.entry_price) / position.entry_price
        
        return loss_pct >= self.stop_loss_pct
    
    def should_take_profit(self, position: Position, current_price: float) -> bool:
        """Check if position should take profit"""
        if position.side == "buy":
            profit_pct = (current_price - position.entry_price) / position.entry_price
        else:
            profit_pct = (position.entry_price - current_price) / position.entry_price
        
        return profit_pct >= self.take_profit_pct

class TradingStrategy:
    """Base class for trading strategies"""
    
    def __init__(self, name: str):
        self.name = name
        self.positions: Dict[str, Position] = {}
        self.trade_history: List[Trade] = []
        self.performance_metrics = {
            'total_trades': 0,
            'winning_trades': 0,
            'total_pnl': 0.0,
            'max_drawdown': 0.0,
            'sharpe_ratio': 0.0
        }
    
    def generate_signal(self, market_data: MarketData, historical_data: List[MarketData]) -> Signal:
        """Generate trading signal"""
        raise NotImplementedError
    
    def update_position(self, symbol: str, side: str, quantity: float, price: float):
        """Update or create position"""
        if symbol in self.positions:
            position = self.positions[symbol]
            # Update existing position (simplified)
            position.current_price = price
            position.unrealized_pnl = self.calculate_pnl(position)
        else:
            # Create new position
            self.positions[symbol] = Position(symbol, side, quantity, price, price)
    
    def calculate_pnl(self, position: Position) -> float:
        """Calculate PnL for position"""
        if position.side == "buy":
            return (position.current_price - position.entry_price) * position.quantity
        else:
            return (position.entry_price - position.current_price) * position.quantity
    
    def close_position(self, symbol: str, exit_price: float) -> float:
        """Close position and return realized PnL"""
        if symbol not in self.positions:
            return 0.0
        
        position = self.positions[symbol]
        position.current_price = exit_price
        realized_pnl = self.calculate_pnl(position)
        position.realized_pnl = realized_pnl
        
        # Update metrics
        self.performance_metrics['total_trades'] += 1
        self.performance_metrics['total_pnl'] += realized_pnl
        if realized_pnl > 0:
            self.performance_metrics['winning_trades'] += 1
        
        del self.positions[symbol]
        return realized_pnl

class MeanReversionStrategy(TradingStrategy):
    """Mean reversion trading strategy"""
    
    def __init__(self, lookback_period: int = 20, threshold: float = 2.0):
        super().__init__("Mean Reversion")
        self.lookback_period = lookback_period
        self.threshold = threshold
        self.price_history: Dict[str, deque] = {}
    
    def generate_signal(self, market_data: MarketData, historical_data: List[MarketData]) -> Signal:
        symbol = market_data.symbol
        
        # Maintain price history
        if symbol not in self.price_history:
            self.price_history[symbol] = deque(maxlen=self.lookback_period * 2)
        
        self.price_history[symbol].append(market_data.mid_price)
        
        if len(self.price_history[symbol]) < self.lookback_period:
            return Signal.HOLD
        
        prices = list(self.price_history[symbol])
        mean_price = statistics.mean(prices[-self.lookback_period:])
        std_price = statistics.stdev(prices[-self.lookback_period:])
        
        current_price = market_data.mid_price
        z_score = (current_price - mean_price) / std_price if std_price > 0 else 0
        
        # Generate signals based on z-score
        if z_score > self.threshold:
            return Signal.SELL  # Price is too high, sell
        elif z_score < -self.threshold:
            return Signal.BUY   # Price is too low, buy
        else:
            return Signal.HOLD

class MomentumStrategy(TradingStrategy):
    """Momentum trading strategy"""
    
    def __init__(self, fast_period: int = 10, slow_period: int = 30):
        super().__init__("Momentum")
        self.fast_period = fast_period
        self.slow_period = slow_period
        self.price_history: Dict[str, deque] = {}
    
    def generate_signal(self, market_data: MarketData, historical_data: List[MarketData]) -> Signal:
        symbol = market_data.symbol
        
        # Maintain price history
        if symbol not in self.price_history:
            self.price_history[symbol] = deque(maxlen=self.slow_period * 2)
        
        self.price_history[symbol].append(market_data.mid_price)
        
        if len(self.price_history[symbol]) < self.slow_period:
            return Signal.HOLD
        
        prices = list(self.price_history[symbol])
        
        # Calculate moving averages
        fast_ma = TechnicalIndicators.sma(prices, self.fast_period)
        slow_ma = TechnicalIndicators.sma(prices, self.slow_period)
        
        # Generate signals based on MA crossover
        if fast_ma > slow_ma:
            # Check if this is a recent crossover
            prev_fast_ma = TechnicalIndicators.sma(prices[:-1], self.fast_period)
            prev_slow_ma = TechnicalIndicators.sma(prices[:-1], self.slow_period)
            
            if prev_fast_ma <= prev_slow_ma:
                return Signal.BUY  # Bullish crossover
            else:
                return Signal.HOLD
        else:
            # Check if this is a recent crossover
            prev_fast_ma = TechnicalIndicators.sma(prices[:-1], self.fast_period)
            prev_slow_ma = TechnicalIndicators.sma(prices[:-1], self.slow_period)
            
            if prev_fast_ma >= prev_slow_ma:
                return Signal.SELL  # Bearish crossover
            else:
                return Signal.HOLD

class MarketMakingStrategy(TradingStrategy):
    """Market making strategy"""
    
    def __init__(self, spread_bps: float = 10, inventory_target: float = 0.5):
        super().__init__("Market Making")
        self.spread_bps = spread_bps / 10000  # Convert basis points to decimal
        self.inventory_target = inventory_target
        self.inventory_skew_factor = 0.1
    
    def generate_signal(self, market_data: MarketData, historical_data: List[MarketData]) -> Signal:
        # Market making doesn't generate directional signals
        return Signal.HOLD
    
    def get_quotes(self, market_data: MarketData, inventory_ratio: float) -> Tuple[float, float]:
        """Get bid and ask quotes"""
        mid_price = market_data.mid_price
        half_spread = mid_price * self.spread_bps / 2
        
        # Adjust quotes based on inventory
        skew_adjustment = self.inventory_skew_factor * (inventory_ratio - self.inventory_target)
        
        bid_price = mid_price - half_spread - (mid_price * skew_adjustment)
        ask_price = mid_price + half_spread - (mid_price * skew_adjustment)
        
        return bid_price, ask_price

class AdvancedTradingBot(ExchangeClient):
    """Advanced trading bot with multiple strategies"""
    
    def __init__(self, client_id: str, strategies: List[TradingStrategy]):
        super().__init__(client_id, Platform.PYTHON)
        self.strategies = strategies
        self.risk_manager = RiskManager()
        self.market_data: Dict[str, MarketData] = {}
        self.historical_data: Dict[str, deque] = {}
        self.active_orders: Dict[int, dict] = {}
        self.performance_history: List[dict] = []
        
        # Portfolio tracking
        self.initial_portfolio_value = 0.0
        self.current_portfolio_value = 0.0
        self.max_portfolio_value = 0.0
        self.drawdown = 0.0
    
    async def handle_orderbook_update(self, message: dict):
        """Handle order book update and store market data"""
        await super().handle_orderbook_update(message)
        
        pair = message["pair"]
        bids = [price/100 for price, _ in message["bids"]]
        asks = [price/100 for price, _ in message["asks"]]
        
        if bids and asks:
            bid_size = message["bids"][0][1] / 1000
            ask_size = message["asks"][0][1] / 1000
            
            market_data = MarketData(
                symbol=pair,
                bid=bids[0],
                ask=asks[0],
                bid_size=bid_size,
                ask_size=ask_size,
                mid_price=(bids[0] + asks[0]) / 2,
                spread=asks[0] - bids[0]
            )
            
            self.market_data[pair] = market_data
            
            # Store historical data
            if pair not in self.historical_data:
                self.historical_data[pair] = deque(maxlen=1000)
            self.historical_data[pair].append(market_data)
    
    def calculate_portfolio_value(self) -> float:
        """Calculate total portfolio value"""
        total_value = 0.0
        
        # Add cash balances
        for asset, balance in self.balances.items():
            if asset == "USDT":
                total_value += balance.available
            elif asset == "BTC":
                if "BTC/USDT" in self.market_data:
                    total_value += balance.available * self.market_data["BTC/USDT"].mid_price
            elif asset == "ETH":
                if "ETH/USDT" in self.market_data:
                    total_value += balance.available * self.market_data["ETH/USDT"].mid_price
        
        # Add position values
        for strategy in self.strategies:
            for position in strategy.positions.values():
                if position.symbol == "BTC/USDT":
                    total_value += position.quantity * position.current_price
                elif position.symbol == "ETH/USDT":
                    total_value += position.quantity * position.current_price
        
        return total_value
    
    def update_performance_metrics(self):
        """Update performance tracking"""
        self.current_portfolio_value = self.calculate_portfolio_value()
        
        if self.initial_portfolio_value == 0:
            self.initial_portfolio_value = self.current_portfolio_value
        
        # Track drawdown
        if self.current_portfolio_value > self.max_portfolio_value:
            self.max_portfolio_value = self.current_portfolio_value
        
        if self.max_portfolio_value > 0:
            self.drawdown = (self.max_portfolio_value - self.current_portfolio_value) / self.max_portfolio_value
        
        # Store performance snapshot
        snapshot = {
            'timestamp': time.time(),
            'portfolio_value': self.current_portfolio_value,
            'drawdown': self.drawdown,
            'total_pnl': self.current_portfolio_value - self.initial_portfolio_value
        }
        
        self.performance_history.append(snapshot)
    
    async def execute_strategy_signals(self):
        """Execute trading signals from all strategies"""
        for strategy in self.strategies:
            for pair in self.pairs:
                if pair not in self.market_data:
                    continue
                
                market_data = self.market_data[pair]
                historical = list(self.historical_data.get(pair, []))
                
                # Generate signal
                signal = strategy.generate_signal(market_data, historical)
                
                if signal != Signal.HOLD:
                    await self.execute_signal(strategy, signal, pair, market_data)
                
                # Handle risk management for existing positions
                await self.manage_positions(strategy, pair, market_data)
    
    async def execute_signal(self, strategy: TradingStrategy, signal: Signal, pair: str, market_data: MarketData):
        """Execute a trading signal"""
        base_asset = pair.split('/')[0]
        quote_asset = pair.split('/')[1]
        
        # Calculate position size
        if quote_asset in self.balances:
            balance = self.balances[quote_asset].available
            volatility = self.calculate_volatility(pair)
            quantity = self.risk_manager.calculate_position_size(balance, market_data.mid_price, volatility)
        else:
            quantity = 0.001  # Default small size
        
        if quantity <= 0:
            return
        
        # Place order based on signal
        if signal == Signal.BUY:
            await self.place_order(OrderSide.BUY, OrderType.MARKET, pair, quantity)
            strategy.update_position(pair, "buy", quantity, market_data.mid_price)
        elif signal == Signal.SELL:
            await self.place_order(OrderSide.SELL, OrderType.MARKET, pair, quantity)
            strategy.update_position(pair, "sell", quantity, market_data.mid_price)
    
    async def manage_positions(self, strategy: TradingStrategy, pair: str, market_data: MarketData):
        """Manage existing positions with risk rules"""
        if pair not in strategy.positions:
            return
        
        position = strategy.positions[pair]
        
        # Check stop loss
        if self.risk_manager.should_stop_loss(position, market_data.mid_price):
            await self.close_position(strategy, pair, market_data.mid_price, "Stop Loss")
        
        # Check take profit
        elif self.risk_manager.should_take_profit(position, market_data.mid_price):
            await self.close_position(strategy, pair, market_data.mid_price, "Take Profit")
    
    async def close_position(self, strategy: TradingStrategy, pair: str, price: float, reason: str):
        """Close a position"""
        if pair not in strategy.positions:
            return
        
        position = strategy.positions[pair]
        
        # Place opposite order
        if position.side == "buy":
            await self.place_order(OrderSide.SELL, OrderType.MARKET, pair, position.quantity)
        else:
            await self.place_order(OrderSide.BUY, OrderType.MARKET, pair, position.quantity)
        
        # Close position in strategy
        realized_pnl = strategy.close_position(pair, price)
        
        logger.info(f"📊 {strategy.name} closed {pair} position at ${price:.2f} - {reason}, PnL: ${realized_pnl:.2f}")
    
    def calculate_volatility(self, pair: str, period: int = 20) -> float:
        """Calculate price volatility"""
        if pair not in self.historical_data or len(self.historical_data[pair]) < period:
            return 0.01  # Default volatility
        
        prices = [data.mid_price for data in list(self.historical_data[pair])[-period:]]
        if len(prices) < 2:
            return 0.01
            
        returns = [(prices[i] - prices[i-1]) / prices[i-1] for i in range(1, len(prices))]
        
        return statistics.stdev(returns) if returns else 0.01
    
    async def advanced_trading_loop(self):
        """Main trading loop with advanced features"""
        logger.info("🤖 Advanced trading bot started")
        
        while self.trading_enabled and self.connected:
            try:
                # Execute strategy signals
                await self.execute_strategy_signals()
                
                # Update performance metrics
                self.update_performance_metrics()
                
                # Log performance
                if random.random() < 0.1:  # Log 10% of the time
                    self.log_performance()
                
                await asyncio.sleep(1)  # Check every second
                
            except Exception as e:
                logger.error(f"Advanced trading loop error: {e}")
                await asyncio.sleep(5)  # Wait before retry
    
    def log_performance(self):
        """Log current performance"""
        total_pnl = self.current_portfolio_value - self.initial_portfolio_value
        total_trades = sum(s.performance_metrics['total_trades'] for s in self.strategies)
        
        if total_trades > 0:
            win_rate = sum(s.performance_metrics['winning_trades'] for s in self.strategies) / total_trades
            pnl_percentage = (total_pnl / self.initial_portfolio_value * 100) if self.initial_portfolio_value > 0 else 0
            drawdown_percentage = self.drawdown * 100
            
            logger.info(f"📊 Portfolio: ${self.current_portfolio_value:.2f}, PnL: ${total_pnl:.2f} ({pnl_percentage:.2f}%)")
            logger.info(f"📈 Trades: {total_trades}, Win Rate: {win_rate*100:.1f}%, Drawdown: {drawdown_percentage:.2f}%")
        else:
            logger.info("📊 No trades yet")
        
        for strategy in self.strategies:
            metrics = strategy.performance_metrics
            if metrics['total_trades'] > 0:
                logger.info(f"🔧 {strategy.name}: {metrics['total_trades']} trades, ${metrics['total_pnl']:.2f} PnL")

async def main():
    """Main function for advanced trading bot"""
    print("🚀 Advanced Crypto Exchange Trading Bot")
    print("=" * 60)
    
    # Create strategies
    strategies = [
        MeanReversionStrategy(lookback_period=20, threshold=1.5),
        MomentumStrategy(fast_period=10, slow_period=30),
        MarketMakingStrategy(spread_bps=15, inventory_target=0.5)
    ]
    
    # Create advanced bot
    bot = AdvancedTradingBot("advanced_bot", strategies)
    
    # Connect to server
    if not await bot.connect():
        logger.error("❌ Failed to connect to server")
        return
    
    # Wait for initial data
    logger.info("⏳ Waiting for initial market data...")
    await asyncio.sleep(5)
    
    # Start trading
    bot.start_trading()
    await bot.advanced_trading_loop()

if __name__ == "__main__":
    asyncio.run(main())

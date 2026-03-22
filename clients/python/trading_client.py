#!/usr/bin/env python3
"""
Crypto Exchange Python Trading Client
Real trading client with WebSocket connection to the exchange server
"""

import asyncio
import json
import websockets
import random
import time
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from enum import Enum
import logging
from config import AppConfig

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class Platform(Enum):
    ANDROID = "Android"
    IOS = "iOS"
    DESKTOP = "Desktop"
    PYTHON = "Python"

class OrderSide(Enum):
    BUY = "buy"
    SELL = "sell"

class OrderType(Enum):
    MARKET = "market"
    LIMIT = "limit"

@dataclass
class Balance:
    asset: str
    available: float
    reserved: float

@dataclass
class OrderBookEntry:
    price: float
    quantity: float

@dataclass
class Trade:
    id: str
    pair: str
    price: float
    quantity: float
    side: str
    timestamp: int

@dataclass
class Order:
    id: int
    side: str
    order_type: str
    price: Optional[float]
    quantity: float
    pair: str
    status: str = "pending"
    filled_quantity: float = 0.0
    remaining_quantity: float = 0.0

class ExchangeClient:
    """WebSocket client for crypto exchange"""
    
    def __init__(self, client_id: str, platform: Platform, config: Optional[AppConfig] = None):
        self.client_id = client_id
        self.platform = platform
        self.config = config or AppConfig.load()
        self.websocket = None
        self.connected = False
        
        # Trading state
        self.balances: Dict[str, Balance] = {}
        self.order_books: Dict[str, Tuple[List[OrderBookEntry], List[OrderBookEntry]]] = {}
        self.trades: List[Trade] = []
        self.orders: Dict[int, Order] = {}
        self.next_order_id = 1
        
        # Trading pairs
        self.pairs = ["BTC/USDT", "ETH/USDT"]
        
        # Trading strategy
        self.trading_enabled = False
        self.trade_interval = 5.0  # seconds
        self.max_order_size = 0.01  # BTC/ETH
        
    async def connect(self) -> bool:
        """Connect to exchange server"""
        try:
            logger.info(f"Connecting {self.platform.value} client {self.client_id} to {self.config.server.url}")
            self.websocket = await websockets.connect(self.config.server.url)
            self.connected = True
            
            # Send identification
            await self.send_message({
                "Identify": {
                    "client_id": self.client_id,
                    "platform": self.platform.value
                }
            })
            
            # Start message handler
            asyncio.create_task(self.message_handler())
            
            logger.info(f"✅ {self.platform.value} client connected successfully")
            return True
            
        except Exception as e:
            logger.error(f"❌ Failed to connect: {e}")
            return False
    
    async def disconnect(self):
        """Disconnect from the server"""
        if self.websocket:
            await self.websocket.close()
            self.connected = False
            logger.info(f"🔌 {self.platform.value} client disconnected")
    
    async def send_message(self, message: dict):
        """Send a message to the server"""
        if self.websocket and self.connected:
            try:
                await self.websocket.send(json.dumps(message))
            except Exception as e:
                logger.error(f"Failed to send message: {e}")
    
    async def message_handler(self):
        """Handle incoming messages from the server"""
        try:
            async for message in self.websocket:
                if message:
                    await self.handle_message(json.loads(message))
        except websockets.exceptions.ConnectionClosed:
            logger.info("Connection closed")
            self.connected = False
        except Exception as e:
            logger.error(f"Message handler error: {e}")
            self.connected = False
    
    async def handle_message(self, message: dict):
        """Process incoming message"""
        msg_type = message.get("type")
        
        if msg_type == "BalanceUpdate":
            await self.handle_balance_update(message)
        elif msg_type == "OrderBookUpdate":
            await self.handle_orderbook_update(message)
        elif msg_type == "Trade":
            await self.handle_trade(message)
        elif msg_type == "OrderUpdate":
            await self.handle_order_update(message)
        elif msg_type == "Response":
            logger.info(f"Server response: {message.get('message')}")
    
    async def handle_balance_update(self, message: dict):
        """Handle balance update"""
        asset = message["asset"]
        available = message["available"] / 100_000_000  # Convert from smallest unit
        reserved = message["reserved"] / 100_000_000
        
        self.balances[asset] = Balance(asset, available, reserved)
        logger.info(f"💰 Balance Update - {asset}: {available:.8f} (Available), {reserved:.8f} (Reserved)")
    
    async def handle_orderbook_update(self, message: dict):
        """Handle order book update"""
        pair = message["pair"]
        
        # Convert bids and asks
        bids = [OrderBookEntry(price/100, qty/1000) for price, qty in message["bids"]]
        asks = [OrderBookEntry(price/100, qty/1000) for price, qty in message["asks"]]
        
        self.order_books[pair] = (bids, asks)
        
        # Log top of book
        if bids and asks:
            best_bid = bids[0]
            best_ask = asks[0]
            spread = best_ask.price - best_bid.price
            logger.info(f"📊 {pair} - Bid: ${best_bid.price:.2f}, Ask: ${best_ask.price:.2f}, Spread: ${spread:.2f}")
    
    async def handle_trade(self, message: dict):
        """Handle trade notification"""
        trade = Trade(
            id=message["id"],
            pair=message["pair"],
            price=message["price"]/100,
            quantity=message["quantity"]/1000,
            side=message["side"],
            timestamp=message["timestamp"]
        )
        
        self.trades.append(trade)
        logger.info(f"🔔 Trade - {trade.side.upper()} {trade.quantity:.6f} {trade.pair} @ ${trade.price:.2f}")
    
    async def handle_order_update(self, message: dict):
        """Handle order status update"""
        order_id = message["id"]
        status = message["status"]
        filled_qty = message["filled_quantity"]/1000
        remaining_qty = message["remaining_quantity"]/1000
        
        if order_id in self.orders:
            order = self.orders[order_id]
            order.status = status
            order.filled_quantity = filled_qty
            order.remaining_quantity = remaining_qty
            logger.info(f"📝 Order {order_id} - Status: {status}, Filled: {filled_qty:.6f}, Remaining: {remaining_qty:.6f}")
    
    async def place_order(self, side: OrderSide, order_type: OrderType, 
                         pair: str, quantity: float, price: Optional[float] = None) -> int:
        """Place an order"""
        order_id = self.next_order_id
        self.next_order_id += 1
        
        # Convert to server units
        quantity_units = int(quantity * 1000)  # Convert to smallest unit
        price_units = int(price * 100) if price else None
        
        message = {
            "type": "PlaceOrder",
            "id": order_id,
            "side": side.value,
            "order_type": order_type.value,
            "price": price_units,
            "quantity": quantity_units,
            "pair": pair
        }
        
        # Store order locally
        order = Order(
            id=order_id,
            side=side.value,
            order_type=order_type.value,
            price=price,
            quantity=quantity,
            pair=pair
        )
        self.orders[order_id] = order
        
        await self.send_message(message)
        logger.info(f"📤 Placed {side.value} {order_type.value} order: {quantity:.6f} {pair}" + 
                   (f" @ ${price:.2f}" if price else ""))
        
        return order_id
    
    async def cancel_order(self, order_id: int):
        """Cancel an order"""
        message = {
            "type": "CancelOrder",
            "id": order_id
        }
        await self.send_message(message)
        logger.info(f"❌ Cancelled order {order_id}")
    
    async def get_balance(self):
        """Request balance update"""
        await self.send_message({"type": "GetBalance"})
    
    async def get_orderbook(self, pair: str):
        """Request order book update"""
        await self.send_message({"type": "GetOrderBook", "pair": pair})
    
    def get_best_price(self, pair: str, side: OrderSide) -> Optional[float]:
        """Get best price for a pair and side"""
        if pair not in self.order_books:
            return None
        
        bids, asks = self.order_books[pair]
        if side == OrderSide.BUY and asks:
            return asks[0].price
        elif side == OrderSide.SELL and bids:
            return bids[0].price
        
        return None
    
    def calculate_order_size(self, pair: str) -> float:
        """Calculate appropriate order size based on balance"""
        base_asset = pair.split('/')[0]
        quote_asset = pair.split('/')[1]
        
        if base_asset in self.balances:
            available_base = self.balances[base_asset].available
            return min(available_base * 0.1, self.max_order_size)  # Use 10% of balance
        elif quote_asset in self.balances:
            available_quote = self.balances[quote_asset].available
            # Estimate based on current price
            price = self.get_best_price(pair, OrderSide.BUY)
            if price:
                return min((available_quote * 0.1) / price, self.max_order_size)
        
        return self.max_order_size
    
    async def trading_bot(self):
        """Simple trading bot logic"""
        logger.info("🤖 Trading bot started")
        
        while self.trading_enabled and self.connected:
            try:
                for pair in self.pairs:
                    # Random trading decision
                    if random.random() < 0.3:  # 30% chance to place order
                        side = random.choice([OrderSide.BUY, OrderSide.SELL])
                        order_type = random.choice([OrderType.MARKET, OrderType.LIMIT])
                        
                        quantity = self.calculate_order_size(pair)
                        price = None
                        
                        if order_type == OrderType.LIMIT:
                            # Get current market price and add spread
                            current_price = self.get_best_price(pair, side)
                            if current_price:
                                spread = random.uniform(-0.01, 0.01)  # ±1% spread
                                price = current_price * (1 + spread)
                        
                        await self.place_order(side, order_type, pair, quantity, price)
                
                await asyncio.sleep(self.trade_interval)
                
            except Exception as e:
                logger.error(f"Trading bot error: {e}")
                await asyncio.sleep(1)
    
    def start_trading(self):
        """Start the trading bot"""
        self.trading_enabled = True
        logger.info("🚀 Trading enabled")
    
    def stop_trading(self):
        """Stop the trading bot"""
        self.trading_enabled = False
        logger.info("🛑 Trading disabled")
    
    def print_status(self):
        """Print current status"""
        print(f"\n📊 {self.platform.value} Client Status")
        print("=" * 50)
        print(f"Connected: {self.connected}")
        print(f"Trading: {self.trading_enabled}")
        print(f"Orders placed: {len(self.orders)}")
        print(f"Trades received: {len(self.trades)}")
        
        print("\n💰 Balances:")
        for asset, balance in self.balances.items():
            print(f"  {asset}: {balance.available:.8f} available, {balance.reserved:.8f} reserved")
        
        print("\n📈 Order Books:")
        for pair, (bids, asks) in self.order_books.items():
            if bids and asks:
                print(f"  {pair}: Bid ${bids[0].price:.2f}, Ask ${asks[0].price:.2f}")

class TradingStrategy:
    """Advanced trading strategies"""
    
    @staticmethod
    def mean_reversion(client: ExchangeClient, pair: str) -> bool:
        """Simple mean reversion strategy"""
        if pair not in client.order_books:
            return False
        
        bids, asks = client.order_books[pair]
        if not bids or not asks:
            return False
        
        mid_price = (bids[0].price + asks[0].price) / 2
        
        # Simple logic: buy when price is low, sell when price is high
        # (In reality, this would use historical data)
        if mid_price < 49000:  # Arbitrary threshold for BTC
            return True  # Buy signal
        elif mid_price > 51000:
            return False  # Sell signal
        
        return None
    
    @staticmethod
    def market_making(client: ExchangeClient, pair: str, spread: float = 0.001):
        """Simple market making strategy"""
        if pair not in client.order_books:
            return
        
        bids, asks = client.order_books[pair]
        if not bids or not asks:
            return
        
        mid_price = (bids[0].price + asks[0].price) / 2
        
        # Place orders around the mid price
        buy_price = mid_price * (1 - spread)
        sell_price = mid_price * (1 + spread)
        
        quantity = client.calculate_order_size(pair) * 0.5  # Smaller size for market making
        
        return [
            (OrderSide.BUY, OrderType.LIMIT, buy_price, quantity),
            (OrderSide.SELL, OrderType.LIMIT, sell_price, quantity)
        ]

async def main():
    """Main function to run the trading client"""
    print("🚀 Crypto Exchange Python Trading Client")
    print("=" * 50)
    
    # Load configuration
    config = AppConfig.load()
    print(f"Loaded configuration: server={config.server.url}")
    
    # Create multiple clients
    clients = [
        ExchangeClient("python_trader_1", Platform.PYTHON, config),
        ExchangeClient("python_trader_2", Platform.PYTHON, config),
        ExchangeClient("python_bot", Platform.PYTHON, config),
    ]
    
    # Connect all clients
    connected_clients = []
    for client in clients:
        if await client.connect():
            connected_clients.append(client)
            await asyncio.sleep(1)  # Stagger connections
    
    if not connected_clients:
        logger.error("❌ No clients connected. Exiting.")
        return
    
    # Wait for initial data
    logger.info("⏳ Waiting for initial data...")
    await asyncio.sleep(3)
    
    # Start trading for bot client
    bot_client = connected_clients[-1]
    bot_client.start_trading()
    asyncio.create_task(bot_client.trading_bot())
    
    # Manual trading for other clients
    manual_clients = connected_clients[:-1]
    
    try:
        while True:
            # Print status every 10 seconds
            for i, client in enumerate(connected_clients):
                client.print_status()
                if i < len(connected_clients) - 1:
                    print("\n" + "-" * 30)
            
            # Place some manual orders
            for client in manual_clients:
                if random.random() < 0.2:  # 20% chance
                    pair = random.choice(client.pairs)
                    side = random.choice([OrderSide.BUY, OrderSide.SELL])
                    quantity = client.calculate_order_size(pair)
                    
                    await client.place_order(side, OrderType.MARKET, pair, quantity)
            
            await asyncio.sleep(10)
            
    except KeyboardInterrupt:
        logger.info("🛑 Shutting down...")
    
    finally:
        # Clean shutdown
        for client in connected_clients:
            client.stop_trading()
            await client.disconnect()

if __name__ == "__main__":
    asyncio.run(main())

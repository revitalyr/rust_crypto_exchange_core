//! Desktop Trading Client with GUI
//! Built with egui for cross-platform desktop trading

use eframe::egui;
use egui::{Color32, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, VecDeque}, time::Instant};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use anyhow::Result;
use chrono::{DateTime, Utc};

// Import semantic types from common crate
use crypto_exchange_common::types::{OrderId, Quantity, Timestamp};
use crypto_exchange_common::order::{OrderSide, OrderType};
use crypto_exchange_common::assets::{Asset, TradingPair};
use crypto_exchange_common::config::AppConfig;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    Identify { 
        client_id: String, 
        platform: String,
    },
    PlaceOrder {
        id: Option<OrderId>,
        side: OrderSide,
        order_type: OrderType,
        price: Option<u64>,
        quantity: Quantity,
        pair: TradingPair,
    },
    CancelOrder { id: OrderId },
    GetBalance,
    GetOrderBook { pair: TradingPair },
    Response {
        success: bool,
        message: String,
        data: Option<serde_json::Value>,
    },
    OrderBookUpdate {
        pair: TradingPair,
        bids: Vec<(u64, Quantity)>,
        asks: Vec<(u64, Quantity)>,
    },
    Trade {
        id: String,
        pair: TradingPair,
        price: u64,
        quantity: Quantity,
        side: OrderSide,
        timestamp: Timestamp,
    },
    BalanceUpdate {
        asset: Asset,
        available: u64,
        reserved: u64,
    },
    OrderUpdate {
        id: OrderId,
        status: String,
        filled_quantity: Quantity,
        remaining_quantity: Quantity,
    },
}

#[derive(Debug, Clone)]
pub struct Balance {
    pub asset: Asset,
    pub available: f64,
    pub reserved: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub id: String,
    pub pair: TradingPair,
    pub price: f64,
    pub quantity: f64,
    pub side: OrderSide,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub quantity: f64,
    pub pair: TradingPair,
    pub status: String,
    pub filled_quantity: f64,
    pub remaining_quantity: f64,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct PriceChart {
    pub prices: VecDeque<f64>,
    pub timestamps: VecDeque<DateTime<Utc>>,
    pub max_points: usize,
}

impl PriceChart {
    pub fn new(max_points: usize) -> Self {
        Self {
            prices: VecDeque::new(),
            timestamps: VecDeque::new(),
            max_points,
        }
    }

    pub fn add_point(&mut self, price: f64, timestamp: DateTime<Utc>) {
        self.prices.push_back(price);
        self.timestamps.push_back(timestamp);

        // Remove old points if we exceed max
        while self.prices.len() > self.max_points {
            self.prices.pop_front();
            self.timestamps.pop_front();
        }
    }

    pub fn get_price_range(&self) -> (f64, f64) {
        if self.prices.is_empty() {
            return (0.0, 0.0);
        }

        let min_price = *self.prices.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max_price = *self.prices.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        (min_price, max_price)
    }
}

pub struct TradingApp {
    // Connection
    connected: bool,
    client_id: String,
    websocket_tx: Option<mpsc::UnboundedSender<WsMessage>>,
    websocket_rx: Option<mpsc::UnboundedReceiver<WsMessage>>,
    config: AppConfig,
    
    // Data
    balances: HashMap<Asset, Balance>,
    order_books: HashMap<TradingPair, (Vec<OrderBookEntry>, Vec<OrderBookEntry>)>,
    trades: VecDeque<Trade>,
    orders: HashMap<OrderId, Order>,
    price_charts: HashMap<TradingPair, PriceChart>,
    
    // UI State
    selected_pair: TradingPair,
    order_side: OrderSide,
    order_type: OrderType,
    order_price: String,
    order_quantity: String,
    show_trade_history: bool,
    show_order_book: bool,
    show_balance_panel: bool,
    show_logs: bool,
    logs: VecDeque<String>,
    
    // Trading
    next_order_id: OrderId,
    auto_trading: bool,
    trade_interval: f64,
    
    // Performance
    total_trades: usize,
    total_volume: f64,
    last_update: Instant,
    
    // Theme
    dark_mode: bool,
}

impl Default for TradingApp {
    fn default() -> Self {
        let config = AppConfig::default();
        Self {
            connected: false,
            client_id: format!("desktop_{}", &uuid::Uuid::new_v4().to_string()[..8]),
            websocket_tx: None,
            websocket_rx: None,
            config,
            balances: HashMap::new(),
            order_books: HashMap::new(),
            trades: VecDeque::new(),
            orders: HashMap::new(),
            price_charts: HashMap::new(),
            selected_pair: TradingPair::new(Asset::BTC, Asset::USDT),
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
            order_price: "50000".to_string(),
            order_quantity: "0.001".to_string(),
            show_trade_history: true,
            show_order_book: true,
            show_balance_panel: true,
            show_logs: true,
            logs: VecDeque::new(),
            next_order_id: 1,
            auto_trading: false,
            trade_interval: 5.0,
            total_trades: 0,
            total_volume: 0.0,
            last_update: Instant::now(),
            dark_mode: true,
        }
    }
}

impl TradingApp {
    pub fn with_config(config: AppConfig) -> Self {
        let mut app = Self::default();
        app.config = config;
        app
    }

    fn start_websocket_connection(&mut self) {
        // Reset connection state
        self.connected = false;
        self.websocket_tx = None;
        self.websocket_rx = None;
        
        self.add_log("🔌 Connecting to server...".to_string());
        
        let (tx, rx) = mpsc::unbounded_channel::<WsMessage>();
        let (app_tx, app_rx) = mpsc::unbounded_channel::<WsMessage>();
        self.websocket_tx = Some(tx);
        self.websocket_rx = Some(app_rx);

        let client_id = self.client_id.clone();
        let config = self.config.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::websocket_task(client_id, rx, app_tx, config).await {
                eprintln!("WebSocket error: {}", e);
            }
        });
    }
    
    fn disconnect(&mut self) {
        self.connected = false;
        self.websocket_tx = None;
        self.websocket_rx = None;
        // Clear data on disconnect
        self.balances.clear();
        self.order_books.clear();
        self.trades.clear();
        self.orders.clear();
        self.add_log("🔴 Disconnected from server".to_string());
    }
    
    fn add_log(&mut self, message: String) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S").to_string();
        self.logs.push_back(format!("[{}] {}", timestamp, message));
        
        // Keep only last 100 logs
        while self.logs.len() > 100 {
            self.logs.pop_front();
        }
    }

    async fn websocket_task(
        client_id: String,
        mut rx: mpsc::UnboundedReceiver<WsMessage>,
        app_tx: mpsc::UnboundedSender<WsMessage>,
        config: AppConfig,
    ) -> Result<()> {
        let url = &config.server.url;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send identification
        let identify_msg = WsMessage::Identify {
            client_id: client_id.clone(),
            platform: "Desktop".to_string(),
        };
        let text = serde_json::to_string(&identify_msg)?;
        write.send(Message::Text(text)).await?;

        // Request initial balance after identification
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let balance_msg = WsMessage::GetBalance;
        let text = serde_json::to_string(&balance_msg)?;
        write.send(Message::Text(text)).await?;

        // Request order book for default pair
        let orderbook_msg = WsMessage::GetOrderBook {
            pair: TradingPair::new(Asset::BTC, Asset::USDT),
        };
        let text = serde_json::to_string(&orderbook_msg)?;
        write.send(Message::Text(text)).await?;

        // Handle messages
        loop {
            tokio::select! {
                // Outgoing messages
                Some(msg) = rx.recv() => {
                    let text = serde_json::to_string(&msg)?;
                    write.send(Message::Text(text)).await?;
                }
                
                // Incoming messages
                Some(msg) = read.next() => {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                // Send message to main thread
                                let _ = app_tx.send(ws_msg.clone());
                                println!("Received: {:?}", ws_msg.get_type());
                                
                                // Mark as connected after first successful message
                                if !client_id.is_empty() {
                                    println!("✅ Connected to server");
                                }
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                
                else => break,
            }
        }

        Ok(())
    }

    fn send_message(&mut self, message: WsMessage) {
        if let Some(tx) = &self.websocket_tx {
            if let Err(e) = tx.send(message) {
                let log_msg = format!("❌ Failed to send message: {:?}", e);
                println!("{}", log_msg);
                self.add_log(log_msg);
            }
        } else {
            let log_msg = "❌ Not connected to server".to_string();
            println!("{}", log_msg);
            self.add_log(log_msg);
        }
    }

    fn place_order(&mut self) {
        let price = if self.order_type == OrderType::Limit {
            self.order_price.parse::<f64>().ok().map(|p| (p * 100.0) as u64)
        } else {
            None
        };

        let quantity = self.order_quantity.parse::<f64>().unwrap_or(0.001) * 1000.0;
        let quantity_units = quantity as u64;

        let order_id = self.next_order_id;
        self.next_order_id += 1;

        let message = WsMessage::PlaceOrder {
            id: Some(order_id),
            side: self.order_side,
            order_type: self.order_type,
            price,
            quantity: quantity_units,
            pair: self.selected_pair.clone(),
        };

        // Store order locally
        let order = Order {
            id: order_id,
            side: self.order_side,
            order_type: self.order_type,
            price: self.order_price.parse::<f64>().ok(),
            quantity: self.order_quantity.parse::<f64>().unwrap_or(0.001),
            pair: self.selected_pair.clone(),
            status: "pending".to_string(),
            filled_quantity: 0.0,
            remaining_quantity: self.order_quantity.parse::<f64>().unwrap_or(0.001),
            created_at: Instant::now(),
        };
        self.orders.insert(order_id, order);

        self.send_message(message);
        let log_msg = format!("📤 Placed {} {} order: {}", self.order_side, self.order_type, self.order_quantity);
        println!("{}", log_msg);
        self.add_log(log_msg);
    }

    fn place_random_order(&mut self) {
        let sides = [OrderSide::Buy, OrderSide::Sell];
        let types = [OrderType::Limit, OrderType::Market];
        
        self.order_side = sides[fastrand::usize(..2)];
        self.order_type = types[fastrand::usize(..2)];
        
        if self.order_type == OrderType::Limit {
            let base_price = if self.selected_pair == TradingPair::new(Asset::BTC, Asset::USDT) { 50000.0 } else { 3000.0 };
            let variation = fastrand::f64() * 1000.0 - 500.0;
            self.order_price = format!("{:.2}", base_price + variation);
        }
        
        self.order_quantity = format!("{:.6}", fastrand::f64() * 0.01 + 0.001);
        
        self.place_order();
    }

    fn show_order_book_panel(&self, ui: &mut egui::Ui) {
        ui.heading("Order Book");
        
        if let Some((bids, asks)) = self.order_books.get(&self.selected_pair) {
            // Asks (red)
            ui.colored_label(Color32::RED, "Asks");
            egui::Grid::new("asks").num_columns(2).striped(true).show(ui, |ui| {
                ui.label("Price");
                ui.label("Quantity");
                ui.end_row();
                
                for ask in asks.iter().take(5).rev() {
                    ui.colored_label(Color32::RED, format!("{:.2}", ask.price));
                    ui.label(format!("{:.6}", ask.quantity));
                    ui.end_row();
                }
            });
            
            ui.separator();
            
            // Bids (green)
            ui.colored_label(Color32::GREEN, "Bids");
            egui::Grid::new("bids").num_columns(2).striped(true).show(ui, |ui| {
                ui.label("Price");
                ui.label("Quantity");
                ui.end_row();
                
                for bid in bids.iter().take(5) {
                    ui.colored_label(Color32::GREEN, format!("{:.2}", bid.price));
                    ui.label(format!("{:.6}", bid.quantity));
                    ui.end_row();
                }
            });
        } else {
            ui.label("No order book data available");
        }
    }

    fn show_trade_history_panel(&self, ui: &mut egui::Ui) {
        ui.heading("Recent Trades");
        
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for trade in self.trades.iter().rev().take(10) {
                ui.horizontal(|ui| {
                    let color = if trade.side == OrderSide::Buy { Color32::GREEN } else { Color32::RED };
                    ui.colored_label(color, &trade.side.to_uppercase());
                    ui.label(format!("{:.6} @ ${:.2}", trade.quantity, trade.price));
                    ui.label(trade.timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, false));
                });
            }
        });
    }

    fn show_price_chart(&mut self, ui: &mut egui::Ui) {
        ui.heading("Price Chart");
        
        if let Some(chart) = self.price_charts.get(&self.selected_pair) {
            if !chart.prices.is_empty() {
                let (min_price, max_price) = chart.get_price_range();
                let price_range = max_price - min_price;
                
                let available_height = ui.available_height() - 50.0;
                let available_width = ui.available_width();
                
                // Draw simple line chart
                let painter = ui.painter();
                let rect = egui::Rect::from_min_size(ui.min_rect().min, Vec2::new(available_width, available_height));
                
                let points: Vec<egui::Pos2> = chart.prices
                    .iter()
                    .enumerate()
                    .map(|(i, &price)| {
                        let x = rect.min.x + (i as f32 / chart.prices.len() as f32) * rect.width();
                        let y = rect.max.y - ((price - min_price) / price_range) as f32 * rect.height();
                        egui::Pos2::new(x, y)
                    })
                    .collect();
                
                if points.len() > 1 {
                    painter.add(egui::Shape::line(points, Stroke::new(2.0, Color32::from_rgb(0, 255, 0))));
                }
                
                // Draw price labels
                ui.label(format!("Min: ${:.2}", min_price));
                ui.label(format!("Max: ${:.2}", max_price));
                ui.label(format!("Current: ${:.2}", chart.prices.back().unwrap_or(&0.0)));
            }
        } else {
            ui.label("No price data available");
        }
    }

    fn show_order_form(&mut self, ui: &mut egui::Ui) {
        ui.heading("Place Order");
        
        ui.horizontal(|ui| {
            ui.label("Pair:");
            egui::ComboBox::from_label("")
                .selected_text(&self.selected_pair.symbol())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_pair, TradingPair::new(Asset::BTC, Asset::USDT), "BTC/USDT");
                    ui.selectable_value(&mut self.selected_pair, TradingPair::new(Asset::ETH, Asset::USDT), "ETH/USDT");
                });
        });
        
        ui.horizontal(|ui| {
            ui.label("Side:");
            ui.radio_value(&mut self.order_side, OrderSide::Buy, "Buy");
            ui.radio_value(&mut self.order_side, OrderSide::Sell, "Sell");
        });
        
        ui.horizontal(|ui| {
            ui.label("Type:");
            ui.radio_value(&mut self.order_type, OrderType::Market, "Market");
            ui.radio_value(&mut self.order_type, OrderType::Limit, "Limit");
        });
        
        if self.order_type == OrderType::Limit {
            ui.horizontal(|ui| {
                ui.label("Price:");
                ui.text_edit_singleline(&mut self.order_price);
            });
        }
        
        ui.horizontal(|ui| {
            ui.label("Quantity:");
            ui.text_edit_singleline(&mut self.order_quantity);
        });
        
        ui.horizontal(|ui| {
            if ui.button("Place Order").clicked() {
                self.place_order();
            }
            
            if ui.button("Clear").clicked() {
                self.order_quantity = "0.001".to_string();
                if self.order_type == OrderType::Limit {
                    self.order_price = "50000".to_string();
                }
            }
        });
    }

    fn show_balance_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Balances");
        
        for (asset, balance) in &self.balances {
            ui.horizontal(|ui| {
                ui.label(asset.symbol());
                ui.label(format!("{:.8}", balance.available));
                ui.label(format!("({:.8} reserved)", balance.reserved));
            });
        }
    }

    fn show_orders_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("My Orders");
        
        let order_ids: Vec<_> = self.orders.keys().take(10).copied().collect();
        for order_id in order_ids.iter().rev() {
            if let Some(order) = self.orders.get(&order_id) {
                let order_side = order.side.clone();
                let order_quantity = order.quantity;
                let order_pair = order.pair.clone();
                let order_status = order.status.clone();
                
                ui.horizontal(|ui| {
                    let color = if order_side == OrderSide::Buy { Color32::GREEN } else { Color32::RED };
                    ui.colored_label(color, &order_side.to_uppercase());
                    ui.label(format!("{} {}", order_quantity, order_pair));
                    ui.label(&order_status);
                    
                    if ui.button("Cancel").clicked() {
                        self.cancel_order(*order_id);
                    }
                });
            }
        }
    }

    fn cancel_order(&mut self, order_id: OrderId) {
        let message = WsMessage::CancelOrder { id: order_id };
        self.send_message(message);
        let log_msg = format!("❌ Cancelled order {}", order_id);
        println!("{}", log_msg);
        self.add_log(log_msg);
    }
    
    fn handle_server_message(&mut self, msg: WsMessage) {
        // Mark as connected when receiving any message from server
        self.connected = true;
        
        match msg {
            WsMessage::Identify { .. } => {
                let log_msg = "✅ Successfully identified with server".to_string();
                println!("{}", log_msg);
                self.add_log(log_msg);
            }
            WsMessage::BalanceUpdate { asset, available, reserved } => {
                // Update balance (simplified for demo)
                let log_msg = format!("💰 Balance update: {} available: {} reserved: {}", asset, available, reserved);
                println!("{}", log_msg);
                self.add_log(log_msg);
            }
            WsMessage::OrderBookUpdate { pair, bids, asks } => {
                // Update order book
                let log_msg = format!("📊 Order book update for {}: {} bids, {} asks", pair, bids.len(), asks.len());
                println!("{}", log_msg);
                self.add_log(log_msg);
            }
            WsMessage::Trade { id: _, pair, price, quantity, side, timestamp: _ } => {
                // Handle trade
                let log_msg = format!("💰 Trade: {} {} {} @ {}", side, quantity, pair, price);
                println!("{}", log_msg);
                self.add_log(log_msg);
            }
            WsMessage::Response { success, message, .. } => {
                // Handle server response
                if success {
                    let log_msg = format!("✅ Server response: {}", message);
                    println!("{}", log_msg);
                    self.add_log(log_msg);
                } else {
                    let log_msg = format!("❌ Server error: {}", message);
                    println!("{}", log_msg);
                    self.add_log(log_msg);
                }
            }
            _ => {
                let log_msg = format!("📨 Received message: {:?}", msg.get_type());
                println!("{}", log_msg);
                self.add_log(log_msg);
            }
        }
    }
}

impl eframe::App for TradingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process websocket messages
        if let Some(ref mut rx) = self.websocket_rx {
            let mut messages = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
            for msg in messages {
                self.handle_server_message(msg);
                // Request repaint when new message arrives
                ctx.request_repaint();
            }
        }
        
        // Simple test UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🚀 Crypto Exchange Client");
            ui.label("If you can see this, egui is working!");
            
            ui.separator();
            
            // Connection status
            let status = if self.connected { "✅ Connected" } else { "❌ Disconnected" };
            ui.label(format!("Status: {}", status));
            ui.label(format!("Client: {}", self.client_id));
            
            ui.separator();
            
            // Connect button
            if ui.button(if self.connected { "Disconnect" } else { "Connect" }).clicked() {
                if self.connected {
                    self.disconnect();
                } else {
                    self.start_websocket_connection();
                }
            }
            
            ui.separator();
            
            // Auto trading
            ui.checkbox(&mut self.auto_trading, "Auto Trading");
            
            ui.separator();
            
            // Show some logs
            ui.heading("Recent Logs:");
            for (i, log) in self.logs.iter().rev().take(10).enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}: {}", i, log));
                    if ui.button("📋").clicked() {
                        // Copy to clipboard
                        ctx.copy_text(log.clone());
                    }
                });
            }
            
            // Add test log
            if ui.button("Add Test Log").clicked() {
                self.add_log("Test log from UI".to_string());
            }
            
            ui.separator();
            
            // Auto trading status
            if self.auto_trading && self.connected {
                ui.colored_label(egui::Color32::GREEN, "🤖 Auto Trading Active");
                
                // Check if it's time to place an order
                if self.last_update.elapsed().as_secs_f64() >= self.trade_interval {
                    self.place_random_order();
                    self.add_log("🤖 Auto-trading: Placed random order".to_string());
                    self.last_update = Instant::now();
                }
            } else if self.auto_trading {
                ui.colored_label(egui::Color32::RED, "🤖 Auto Trading - Not Connected");
            } else {
                ui.label("Auto Trading: Disabled");
            }
        });
    }
}

// Extension to get message type for debugging
impl WsMessage {
    fn get_type(&self) -> &'static str {
        match self {
            WsMessage::Identify { .. } => "Identify",
            WsMessage::PlaceOrder { .. } => "PlaceOrder",
            WsMessage::CancelOrder { .. } => "CancelOrder",
            WsMessage::GetBalance => "GetBalance",
            WsMessage::GetOrderBook { .. } => "GetOrderBook",
            WsMessage::Response { .. } => "Response",
            WsMessage::OrderBookUpdate { .. } => "OrderBookUpdate",
            WsMessage::Trade { .. } => "Trade",
            WsMessage::BalanceUpdate { .. } => "BalanceUpdate",
            WsMessage::OrderUpdate { .. } => "OrderUpdate",
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // Load configuration
    let config = match AppConfig::load() {
        Ok(config) => {
            println!("Loaded configuration from file");
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}. Using defaults.", e);
            AppConfig::default()
        }
    };

    let app = TradingApp::with_config(config);
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Crypto Exchange Desktop Client",
        options,
        Box::new(|_cc| Box::new(app)),
    )
}

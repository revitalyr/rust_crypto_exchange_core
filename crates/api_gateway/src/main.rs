//! Multi-client crypto exchange demo server
//! Supports Android, iOS, and Desktop clients via WebSocket

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::{get, Router},
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn, debug};
use tracing_subscriber;
use uuid::Uuid;

use crypto_exchange_common::{
    assets::Asset,
    order::{OrderSide, OrderType},
    price::Price,
    types::{OrderId, UserId, Timestamp, symbols},
    Balance, Quantity,
};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Client identification
    Identify { 
        client_id: String, 
        platform: Platform 
    },
    /// Order placement request
    PlaceOrder {
        id: Option<OrderId>,
        side: String,
        order_type: String,
        price: Option<u64>,
        quantity: Quantity,
        pair: String,
    },
    /// Order cancellation
    CancelOrder { id: OrderId },
    /// Get account balance
    GetBalance,
    /// Get order book
    GetOrderBook { pair: String },
    /// Server responses
    Response {
        success: bool,
        message: String,
        data: Option<serde_json::Value>,
    },
    /// Order book update
    OrderBookUpdate {
        pair: String,
        bids: Vec<(u64, Quantity)>,
        asks: Vec<(u64, Quantity)>,
    },
    /// Trade notification
    Trade {
        id: String,
        pair: String,
        price: u64,
        quantity: Quantity,
        side: String,
        timestamp: Timestamp,
    },
    /// Balance update
    BalanceUpdate {
        asset: String,
        available: Balance,
        reserved: Balance,
    },
    /// Order status update
    OrderUpdate {
        id: OrderId,
        status: String,
        filled_quantity: Quantity,
        remaining_quantity: Quantity,
    },
}

/// Extension to get message type for debugging
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

/// Client platform types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    Android,
    IOS,
    Desktop,
    Web,
}

/// Connected client info
#[derive(Debug, Clone)]
pub struct ConnectedClient {
    pub id: String,
    pub platform: Platform,
    pub user_id: UserId,
    pub sender: broadcast::Sender<WsMessage>,
}

/// Exchange server state
#[derive(Debug)]
pub struct ExchangeServer {
    clients: Arc<DashMap<String, ConnectedClient>>,
    event_sender: broadcast::Sender<WsMessage>,
    order_books: HashMap<String, OrderBook>,
    user_balances: HashMap<UserId, HashMap<Asset, (Balance, Balance)>>, // (available, reserved)
    next_order_id: OrderId,
    next_user_id: UserId,
}

/// Simple order book for demo
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: Vec<(u64, Quantity)>,
    pub asks: Vec<(u64, Quantity)>,
}

impl ExchangeServer {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            clients: Arc::new(DashMap::new()),
            event_sender,
            order_books: HashMap::new(),
            user_balances: HashMap::new(),
            next_order_id: 1,
            next_user_id: 1,
        }
    }

    pub fn initialize_demo_data(&mut self) {
        // Initialize order books for popular pairs
        let btc_usdt = OrderBook {
            bids: vec![
                (49900, 1000),
                (49800, 500),
                (49700, 2000),
            ],
            asks: vec![
                (50100, 800),
                (50200, 1200),
                (50300, 600),
            ],
        };
        self.order_books.insert(symbols::BTC_USDT.to_string(), btc_usdt);

        let eth_usdt = OrderBook {
            bids: vec![
                (2990, 500),
                (2985, 800),
                (2980, 300),
            ],
            asks: vec![
                (3010, 400),
                (3015, 700),
                (3020, 900),
            ],
        };
        self.order_books.insert(symbols::ETH_USDT.to_string(), eth_usdt);

        // Initialize demo users with balances
        let demo_users = vec![
            ("android_user", Platform::Android),
            ("ios_user", Platform::IOS),
            ("desktop_user", Platform::Desktop),
        ];

        for (_username, _platform) in demo_users {
            let user_id = self.next_user_id;
            self.next_user_id += 1;

            let mut balances = HashMap::new();
            balances.insert(Asset::BTC, (1_000_000_000, 0)); // 10 BTC
            balances.insert(Asset::ETH, (10_000_000_000_000, 0)); // 10 ETH
            balances.insert(Asset::USDT, (100_000_000_000_000, 0)); // 100,000 USDT

            self.user_balances.insert(user_id, balances);
        }
    }

    pub fn register_client(&mut self, client_id: String, platform: Platform) -> UserId {
        let user_id = self.next_user_id;
        self.next_user_id += 1;

        // Initialize balance for new user
        let mut balances = HashMap::new();
        balances.insert(Asset::BTC, (5_000_000_000, 0)); // 5 BTC
        balances.insert(Asset::ETH, (5_000_000_000_000, 0)); // 5 ETH
        balances.insert(Asset::USDT, (50_000_000_000_000, 0)); // 50,000 USDT

        self.user_balances.insert(user_id, balances);

        info!("Registered client: {} ({:?}) with user_id: {}", client_id, platform, user_id);
        user_id
    }

    pub async fn handle_message(&mut self, _client_id: String, message: WsMessage) -> Vec<WsMessage> {
        let mut responses = Vec::new();

        match message {
            WsMessage::Identify { client_id: id, platform } => {
                let user_id = self.register_client(id.clone(), platform);
                
                // Send initial balance
                if let Some(balances) = self.user_balances.get(&user_id) {
                    for (asset, (available, _reserved)) in balances {
                        responses.push(WsMessage::BalanceUpdate {
                            asset: format!("{:?}", asset),
                            available: *available,
                            reserved: 0,
                        });
                    }
                }

                // Send initial order books
                for (pair, order_book) in &self.order_books {
                    responses.push(WsMessage::OrderBookUpdate {
                        pair: pair.clone(),
                        bids: order_book.bids.clone(),
                        asks: order_book.asks.clone(),
                    });
                }
            }

            WsMessage::PlaceOrder { id, side, order_type, price, quantity, pair } => {
                if let Some(order_book) = self.order_books.get_mut(&pair) {
                    let order_id = id.unwrap_or_else(|| {
                        let id = self.next_order_id;
                        self.next_order_id += 1;
                        id
                    });

                    // Simple order matching logic for demo
                    let side = if side == "buy" { OrderSide::Buy } else { OrderSide::Sell };
                    let order_type = if order_type == "limit" { OrderType::Limit } else { OrderType::Market };
                    let _price = price.map(Price::new);

                    // Simulate order execution
                    if side == OrderSide::Buy && order_type == OrderType::Market {
                        if let Some((ask_price, ask_quantity)) = order_book.asks.first() {
                            let trade_quantity = quantity.min(*ask_quantity);
                            
                            info!("💰 Trade executed: BUY {} @ {} (filled: {}/{})", quantity, ask_price, trade_quantity, quantity);
                            
                            // Broadcast trade
                            let trade_msg = WsMessage::Trade {
                                id: Uuid::new_v4().to_string(),
                                pair: pair.clone(),
                                price: *ask_price,
                                quantity: trade_quantity,
                                side: side.to_string(),
                                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                            };

                            if let Err(e) = self.event_sender.send(trade_msg.clone()) {
                                warn!("Failed to broadcast trade: {}", e);
                            } else {
                                info!("📡 Trade broadcasted to all clients");
                            }

                            responses.push(trade_msg);

                            // Update order book (simplified)
                            if *ask_quantity == trade_quantity {
                                order_book.asks.remove(0);
                            } else {
                                order_book.asks[0].1 -= trade_quantity;
                            }

                            // Broadcast order book update
                            let book_update = WsMessage::OrderBookUpdate {
                                pair: pair.clone(),
                                bids: order_book.bids.clone(),
                                asks: order_book.asks.clone(),
                            };

                            if let Err(e) = self.event_sender.send(book_update.clone()) {
                                warn!("Failed to broadcast order book update: {}", e);
                            }
                        }
                    }

                    // Send order confirmation
                    responses.push(WsMessage::OrderUpdate {
                        id: order_id,
                        status: "filled".to_string(),
                        filled_quantity: quantity,
                        remaining_quantity: 0,
                    });
                }
            }

            WsMessage::GetOrderBook { pair } => {
                if let Some(order_book) = self.order_books.get(&pair) {
                    responses.push(WsMessage::OrderBookUpdate {
                        pair,
                        bids: order_book.bids.clone(),
                        asks: order_book.asks.clone(),
                    });
                }
            }

            WsMessage::GetBalance => {
                // Find user_id for this client (simplified for demo)
                for (_user_id, balances) in &self.user_balances {
                    for (asset, (available, reserved)) in balances {
                        responses.push(WsMessage::BalanceUpdate {
                            asset: format!("{:?}", asset),
                            available: *available,
                            reserved: *reserved,
                        });
                    }
                    break; // Just send first user's balance for demo
                }
            }

            _ => {
                responses.push(WsMessage::Response {
                    success: false,
                    message: "Message not implemented".to_string(),
                    data: None,
                });
            }
        }

        responses
    }
}

/// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(server): State<Arc<tokio::sync::Mutex<ExchangeServer>>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, server))
}

async fn handle_socket(socket: WebSocket, server: Arc<tokio::sync::Mutex<ExchangeServer>>) {
    let (mut sender, mut receiver) = socket.split();
    let mut client_id: Option<String> = None;

    info!("🔌 New WebSocket connection established");

    // Subscribe to server events
    let mut event_rx = {
        let server = server.lock().await;
        server.event_sender.subscribe()
    };

    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        info!("📨 Received message: {}", text);
                        
                        if let Ok(ws_message) = serde_json::from_str::<WsMessage>(&text) {
                            match &ws_message {
                                WsMessage::Identify { client_id: id, platform } => {
                                    client_id = Some(id.clone());
                                    info!("👤 Client identified: {} ({:?})", id, platform);
                                }
                                _ => {
                                    // Only process other messages if client is identified
                                    if client_id.is_none() {
                                        warn!("🚫 Message from unidentified client: {:?}", ws_message.get_type());
                                        continue;
                                    }
                                    
                                    // Process message normally
                                    let mut server = server.lock().await;
                                    let responses = server.handle_message(client_id.clone().unwrap_or_default(), ws_message).await;
                                    
                                    // Send responses back to client
                                    for response in responses {
                                        if let Ok(text) = serde_json::to_string(&response) {
                                            let _ = sender.send(Message::Text(text)).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        if let Some(id) = &client_id {
                            info!("👋 Client {} disconnected", id);
                        } else {
                            info!("👋 Unknown client disconnected");
                        }
                        break;
                    }
                    Err(e) => {
                        warn!("❌ WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle server events (broadcast to all clients)
            Ok(event) = event_rx.recv() => {
                debug!("📡 Broadcasting event: {:?}", event.get_type());
                if let Ok(text) = serde_json::to_string(&event) {
                    let _ = sender.send(Message::Text(text)).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with DEBUG level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("🚀 Starting Crypto Exchange Demo Server");
    info!("📡 WebSocket: ws://127.0.0.1:8080/ws");
    info!("🌐 Web UI: http://127.0.0.1:8080");

    // Create and initialize server
    let mut server = ExchangeServer::new();
    server.initialize_demo_data();
    info!("📊 Demo data initialized with order books and user balances");
    
    let server = Arc::new(tokio::sync::Mutex::new(server));

    // Build router
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(server);

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("� Server listening on 127.0.0.1:8080");
    info!("⚡ Ready for client connections!");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

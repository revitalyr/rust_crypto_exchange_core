//
//  TradingClient.swift
//  CryptoExchange
//
//  Created on 2024.
//  Copyright © 2024 Crypto Exchange. All rights reserved.
//

import SwiftUI
import Foundation
import Starscream

// MARK: - Data Models

struct Balance: Identifiable, Codable {
    let id = UUID()
    let asset: String
    let available: Double
    let reserved: Double
}

struct OrderBookEntry: Codable {
    let price: Double
    let quantity: Double
}

struct Trade: Identifiable, Codable {
    let id: String
    let pair: String
    let price: Double
    let quantity: Double
    let side: String
    let timestamp: Date
}

struct Order: Identifiable, Codable {
    let id: Int64
    let side: String
    let orderType: String
    let price: Double?
    let quantity: Double
    let pair: String
    let status: String
    let filledQuantity: Double
    let remainingQuantity: Double
}

// MARK: - WebSocket Messages

enum WebSocketMessage: Codable {
    case identify(clientId: String, platform: String)
    case placeOrder(id: Int64?, side: String, orderType: String, price: Int64?, quantity: Int64, pair: String)
    case cancelOrder(id: Int64)
    case getBalance
    case getOrderBook(pair: String)
    case response(success: Bool, message: String, data: String?)
    case orderBookUpdate(pair: String, bids: [OrderBookEntry], asks: [OrderBookEntry])
    case trade(id: String, pair: String, price: Int64, quantity: Int64, side: String, timestamp: Int64)
    case balanceUpdate(asset: String, available: Int64, reserved: Int64)
    case orderUpdate(id: Int64, status: String, filledQuantity: Int64, remainingQuantity: Int64)
    
    enum CodingKeys: String, CodingKey {
        case type
        case client_id
        case platform
        case id
        case side
        case order_type
        case price
        case quantity
        case pair
        case success
        case message
        case data
        case bids
        case asks
        case timestamp
        case asset
        case available
        case reserved
        case filled_quantity
        case remaining_quantity
    }
    
    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        
        switch type {
        case "Identify":
            let clientId = try container.decode(String.self, forKey: .client_id)
            let platform = try container.decode(String.self, forKey: .platform)
            self = .identify(clientId: clientId, platform: platform)
        case "PlaceOrder":
            let id = try container.decodeIfPresent(Int64.self, forKey: .id)
            let side = try container.decode(String.self, forKey: .side)
            let orderType = try container.decode(String.self, forKey: .order_type)
            let price = try container.decodeIfPresent(Int64.self, forKey: .price)
            let quantity = try container.decode(Int64.self, forKey: .quantity)
            let pair = try container.decode(String.self, forKey: .pair)
            self = .placeOrder(id: id, side: side, orderType: orderType, price: price, quantity: quantity, pair: pair)
        case "CancelOrder":
            let id = try container.decode(Int64.self, forKey: .id)
            self = .cancelOrder(id: id)
        case "GetBalance":
            self = .getBalance
        case "GetOrderBook":
            let pair = try container.decode(String.self, forKey: .pair)
            self = .getOrderBook(pair: pair)
        case "Response":
            let success = try container.decode(Bool.self, forKey: .success)
            let message = try container.decode(String.self, forKey: .message)
            let data = try container.decodeIfPresent(String.self, forKey: .data)
            self = .response(success: success, message: message, data: data)
        case "OrderBookUpdate":
            let pair = try container.decode(String.self, forKey: .pair)
            let bids = try container.decode([OrderBookEntry].self, forKey: .bids)
            let asks = try container.decode([OrderBookEntry].self, forKey: .asks)
            self = .orderBookUpdate(pair: pair, bids: bids, asks: asks)
        case "Trade":
            let id = try container.decode(String.self, forKey: .id)
            let pair = try container.decode(String.self, forKey: .pair)
            let price = try container.decode(Int64.self, forKey: .price)
            let quantity = try container.decode(Int64.self, forKey: .quantity)
            let side = try container.decode(String.self, forKey: .side)
            let timestamp = try container.decode(Int64.self, forKey: .timestamp)
            self = .trade(id: id, pair: pair, price: price, quantity: quantity, side: side, timestamp: timestamp)
        case "BalanceUpdate":
            let asset = try container.decode(String.self, forKey: .asset)
            let available = try container.decode(Int64.self, forKey: .available)
            let reserved = try container.decode(Int64.self, forKey: .reserved)
            self = .balanceUpdate(asset: asset, available: available, reserved: reserved)
        case "OrderUpdate":
            let id = try container.decode(Int64.self, forKey: .id)
            let status = try container.decode(String.self, forKey: .status)
            let filledQuantity = try container.decode(Int64.self, forKey: .filled_quantity)
            let remainingQuantity = try container.decode(Int64.self, forKey: .remaining_quantity)
            self = .orderUpdate(id: id, status: status, filledQuantity: filledQuantity, remainingQuantity: remainingQuantity)
        default:
            throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Unknown message type"))
        }
    }
    
    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        
        switch self {
        case .identify(let clientId, let platform):
            try container.encode("Identify", forKey: .type)
            try container.encode(clientId, forKey: .client_id)
            try container.encode(platform, forKey: .platform)
        case .placeOrder(let id, let side, let orderType, let price, let quantity, let pair):
            try container.encode("PlaceOrder", forKey: .type)
            try container.encodeIfPresent(id, forKey: .id)
            try container.encode(side, forKey: .side)
            try container.encode(orderType, forKey: .order_type)
            try container.encodeIfPresent(price, forKey: .price)
            try container.encode(quantity, forKey: .quantity)
            try container.encode(pair, forKey: .pair)
        case .cancelOrder(let id):
            try container.encode("CancelOrder", forKey: .type)
            try container.encode(id, forKey: .id)
        case .getBalance:
            try container.encode("GetBalance", forKey: .type)
        case .getOrderBook(let pair):
            try container.encode("GetOrderBook", forKey: .type)
            try container.encode(pair, forKey: .pair)
        case .response(let success, let message, let data):
            try container.encode("Response", forKey: .type)
            try container.encode(success, forKey: .success)
            try container.encode(message, forKey: .message)
            try container.encodeIfPresent(data, forKey: .data)
        case .orderBookUpdate(let pair, let bids, let asks):
            try container.encode("OrderBookUpdate", forKey: .type)
            try container.encode(pair, forKey: .pair)
            try container.encode(bids, forKey: .bids)
            try container.encode(asks, forKey: .asks)
        case .trade(let id, let pair, let price, let quantity, let side, let timestamp):
            try container.encode("Trade", forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(pair, forKey: .pair)
            try container.encode(price, forKey: .price)
            try container.encode(quantity, forKey: .quantity)
            try container.encode(side, forKey: .side)
            try container.encode(timestamp, forKey: .timestamp)
        case .balanceUpdate(let asset, let available, let reserved):
            try container.encode("BalanceUpdate", forKey: .type)
            try container.encode(asset, forKey: .asset)
            try container.encode(available, forKey: .available)
            try container.encode(reserved, forKey: .reserved)
        case .orderUpdate(let id, let status, let filledQuantity, let remainingQuantity):
            try container.encode("OrderUpdate", forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(status, forKey: .status)
            try container.encode(filledQuantity, forKey: .filled_quantity)
            try container.encode(remainingQuantity, forKey: .remaining_quantity)
        }
    }
}

// MARK: - Trading Client

class TradingClient: ObservableObject {
    @Published var isConnected = false
    @Published var balances: [Balance] = []
    @Published var orderBooks: [String: (bids: [OrderBookEntry], asks: [OrderBookEntry])] = [:]
    @Published var trades: [Trade] = []
    @Published var orders: [Order] = []
    
    private var socket: WebSocket?
    private var nextOrderId: Int64 = 1
    private let clientId = "ios_\(UUID().uuidString.prefix(8))"
    
    // UI State
    @Published var selectedPair = "BTC/USDT"
    @Published var orderSide = "buy"
    @Published var orderType = "market"
    @Published var orderPrice = "50000"
    @Published var orderQuantity = "0.001"
    @Published var autoTrading = false
    
    private var autoTradingTimer: Timer?
    
    init() {
        connectWebSocket()
    }
    
    deinit {
        disconnectWebSocket()
    }
    
    // MARK: - WebSocket Connection
    
    private func connectWebSocket() {
        var request = URLRequest(url: URL(string: "ws://127.0.0.1:8080/ws")!)
        request.timeoutInterval = 5
        socket = WebSocket(request: request)
        socket?.delegate = self
        socket?.connect()
    }
    
    private func disconnectWebSocket() {
        socket?.disconnect()
        socket = nil
        autoTradingTimer?.invalidate()
        autoTradingTimer = nil
    }
    
    private func sendMessage(_ message: WebSocketMessage) {
        do {
            let data = try JSONEncoder().encode(message)
            if let string = String(data: data, encoding: .utf8) {
                socket?.write(string: string)
                print("Sent: \(string)")
            }
        } catch {
            print("Failed to encode message: \(error)")
        }
    }
    
    // MARK: - Trading Operations
    
    func placeOrder() {
        let price = orderType == "limit" ? Int64((Double(orderPrice) ?? 0) * 100) : nil
        let quantity = Int64((Double(orderQuantity) ?? 0) * 1000)
        
        let message = WebSocketMessage.placeOrder(
            id: nextOrderId,
            side: orderSide,
            orderType: orderType,
            price: price,
            quantity: quantity,
            pair: selectedPair
        )
        
        nextOrderId += 1
        sendMessage(message)
        
        print("Placed \(orderSide) \(orderType) order: \(orderQuantity) \(selectedPair)")
    }
    
    func cancelOrder(id: Int64) {
        let message = WebSocketMessage.cancelOrder(id: id)
        sendMessage(message)
        print("Cancelled order \(id)")
    }
    
    func placeRandomOrder() {
        let sides = ["buy", "sell"]
        let types = ["market", "limit"]
        
        orderSide = sides.randomElement() ?? "buy"
        orderType = types.randomElement() ?? "market"
        
        if orderType == "limit" {
            let basePrice = selectedPair == "BTC/USDT" ? 50000.0 : 3000.0
            let variation = Double.random(in: -1000...1000)
            orderPrice = String(format: "%.2f", basePrice + variation)
        }
        
        orderQuantity = String(format: "%.6f", Double.random(in: 0.001...0.01))
        placeOrder()
    }
    
    func toggleAutoTrading() {
        autoTrading.toggle()
        
        if autoTrading {
            startAutoTrading()
        } else {
            stopAutoTrading()
        }
    }
    
    private func startAutoTrading() {
        autoTradingTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { _ in
            if self.autoTrading && self.isConnected {
                self.placeRandomOrder()
            }
        }
    }
    
    private func stopAutoTrading() {
        autoTradingTimer?.invalidate()
        autoTradingTimer = nil
    }
    
    // MARK: - Message Handling
    
    private func handleBalanceUpdate(_ message: WebSocketMessage) {
        if case .balanceUpdate(let asset, let available, let reserved) = message {
            let balance = Balance(
                asset: asset,
                available: Double(available) / 100_000_000,
                reserved: Double(reserved) / 100_000_000
            )
            
            DispatchQueue.main.async {
                if let index = self.balances.firstIndex(where: { $0.asset == asset }) {
                    self.balances[index] = balance
                } else {
                    self.balances.append(balance)
                }
            }
            
            print("Balance Update - \(asset): \(balance.available) available, \(balance.reserved) reserved")
        }
    }
    
    private func handleOrderBookUpdate(_ message: WebSocketMessage) {
        if case .orderBookUpdate(let pair, let bids, let asks) = message {
            DispatchQueue.main.async {
                self.orderBooks[pair] = (bids: bids, asks: asks)
            }
            
            if let bid = bids.first, let ask = asks.first {
                print("OrderBook Update - \(pair): Bid $\(bid.price), Ask $\(ask.price)")
            }
        }
    }
    
    private func handleTrade(_ message: WebSocketMessage) {
        if case .trade(let id, let pair, let price, let quantity, let side, let timestamp) = message {
            let trade = Trade(
                id: id,
                pair: pair,
                price: Double(price) / 100,
                quantity: Double(quantity) / 1000,
                side: side,
                timestamp: Date(timeIntervalSince1970: Double(timestamp) / 1_000_000_000)
            )
            
            DispatchQueue.main.async {
                self.trades.insert(trade, at: 0)
                if self.trades.count > 50 {
                    self.trades.removeLast()
                }
            }
            
            print("Trade - \(side.uppercased()) \(trade.quantity) \(pair) @ $\(trade.price)")
        }
    }
    
    private func handleOrderUpdate(_ message: WebSocketMessage) {
        if case .orderUpdate(let id, let status, let filledQuantity, let remainingQuantity) = message {
            DispatchQueue.main.async {
                if let index = self.orders.firstIndex(where: { $0.id == id }) {
                    var order = self.orders[index]
                    order = Order(
                        id: order.id,
                        side: order.side,
                        orderType: order.orderType,
                        price: order.price,
                        quantity: order.quantity,
                        pair: order.pair,
                        status: status,
                        filledQuantity: Double(filledQuantity) / 1000,
                        remainingQuantity: Double(remainingQuantity) / 1000
                    )
                    self.orders[index] = order
                }
            }
            
            print("Order Update - \(id): Status \(status), Filled \(Double(filledQuantity) / 1000)")
        }
    }
}

// MARK: - WebSocketDelegate

extension TradingClient: WebSocketDelegate {
    func didReceive(event: WebSocketEvent, client: WebSocket) {
        switch event {
        case .connected:
            print("WebSocket connected")
            DispatchQueue.main.async {
                self.isConnected = true
            }
            
            // Send identification
            let message = WebSocketMessage.identify(clientId: clientId, platform: "iOS")
            sendMessage(message)
            
        case .disconnected(let reason, let code):
            print("WebSocket disconnected: \(reason) with code: \(code)")
            DispatchQueue.main.async {
                self.isConnected = false
            }
            
        case .text(let string):
            handleTextMessage(string)
            
        case .binary(let data):
            print("Received binary data: \(data.count) bytes")
            
        case .ping(_):
            break
            
        case .pong(_):
            break
            
        case .viabilityChanged(_):
            break
            
        case .reconnectSuggested(_):
            if let socket = socket {
                socket.disconnect()
                connectWebSocket()
            }
            
        case .cancelled:
            print("WebSocket cancelled")
            DispatchQueue.main.async {
                self.isConnected = false
            }
            
        case .error(let error):
            print("WebSocket error: \(error?.localizedDescription ?? "Unknown error")")
            DispatchQueue.main.async {
                self.isConnected = false
            }
        }
    }
    
    private func handleTextMessage(_ string: String) {
        do {
            let data = string.data(using: .utf8) ?? Data()
            let message = try JSONDecoder().decode(WebSocketMessage.self, from: data)
            
            switch message {
            case .balanceUpdate:
                handleBalanceUpdate(message)
            case .orderBookUpdate:
                handleOrderBookUpdate(message)
            case .trade:
                handleTrade(message)
            case .orderUpdate:
                handleOrderUpdate(message)
            case .response(let success, let message, _):
                print("Server response: \(message)")
            default:
                print("Received message: \(message)")
            }
        } catch {
            print("Failed to decode message: \(error)")
        }
    }
}

// MARK: - SwiftUI View

struct TradingView: View {
    @StateObject private var tradingClient = TradingClient()
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 16) {
                    // Connection Status
                    connectionStatusView
                    
                    // Order Form
                    orderFormView
                    
                    // Order Book
                    orderBookView
                    
                    // Balances
                    balancesView
                    
                    // Recent Trades
                    tradesView
                }
                .padding()
            }
            .navigationTitle("📱 Crypto Exchange")
            .navigationBarTitleDisplayMode(.inline)
        }
    }
    
    private var connectionStatusView: some View {
        HStack {
            Image(systemName: tradingClient.isConnected ? "wifi" : "wifi.slash")
                .foregroundColor(tradingClient.isConnected ? .green : .red)
            
            Text(tradingClient.isConnected ? "Connected" : "Disconnected")
                .foregroundColor(tradingClient.isConnected ? .green : .red)
            
            Spacer()
            
            Button(action: tradingClient.toggleAutoTrading) {
                Text("Auto Trade: \(tradingClient.autoTrading ? "ON" : "OFF")")
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(tradingClient.autoTrading ? Color.green : Color.gray)
                    .foregroundColor(.white)
                    .cornerRadius(8)
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(10)
    }
    
    private var orderFormView: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Place Order")
                .font(.headline)
                .fontWeight(.bold)
            
            // Trading Pair
            HStack {
                Text("Pair:")
                TextField("BTC/USDT", text: $tradingClient.orderPrice)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .frame(width: 120)
            }
            
            // Order Side
            HStack {
                Text("Side:")
                Picker("Side", selection: $tradingClient.orderSide) {
                    Text("Buy").tag("buy")
                    Text("Sell").tag("sell")
                }
                .pickerStyle(SegmentedPickerStyle())
            }
            
            // Order Type
            HStack {
                Text("Type:")
                Picker("Type", selection: $tradingClient.orderType) {
                    Text("Market").tag("market")
                    Text("Limit").tag("limit")
                }
                .pickerStyle(SegmentedPickerStyle())
            }
            
            // Price (for limit orders)
            if tradingClient.orderType == "limit" {
                HStack {
                    Text("Price:")
                    TextField("50000", text: $tradingClient.orderPrice)
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                        .keyboardType(.decimalPad)
                }
            }
            
            // Quantity
            HStack {
                Text("Quantity:")
                TextField("0.001", text: $tradingClient.orderQuantity)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .keyboardType(.decimalPad)
            }
            
            // Buttons
            HStack {
                Button(action: tradingClient.placeOrder) {
                    Text("Place \(tradingClient.orderSide.uppercased()) Order")
                        .foregroundColor(.white)
                        .frame(maxWidth: .infinity)
                }
                .background(tradingClient.orderSide == "buy" ? Color.green : Color.red)
                .cornerRadius(8)
                
                Button("Clear") {
                    tradingClient.orderQuantity = "0.001"
                    tradingClient.orderPrice = "50000"
                }
                .frame(maxWidth: .infinity)
                .background(Color.gray)
                .foregroundColor(.white)
                .cornerRadius(8)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }
    
    private var orderBookView: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Order Book")
                .font(.headline)
                .fontWeight(.bold)
            
            if let orderBook = tradingClient.orderBooks[tradingClient.selectedPair] {
                LazyColumn {
                    // Asks
                    ForEach(orderBook.asks.reversed().prefix(5), id: \.price) { ask in
                        HStack {
                            Text("$\(ask.price, specifier: "%.2f")")
                                .foregroundColor(.red)
                            Spacer()
                            Text("\(ask.quantity, specifier: "%.6f")")
                                .foregroundColor(.red)
                        }
                    }
                    
                    // Spread
                    if let bid = orderBook.bids.first, let ask = orderBook.asks.first {
                        HStack {
                            Spacer()
                            Text("Spread: $\(String(format: "%.2f", ask.price - bid.price))")
                                .foregroundColor(.gray)
                            Spacer()
                        }
                    }
                    
                    // Bids
                    ForEach(orderBook.bids.prefix(5), id: \.price) { bid in
                        HStack {
                            Text("$\(bid.price, specifier: "%.2f")")
                                .foregroundColor(.green)
                            Spacer()
                            Text("\(bid.quantity, specifier: "%.6f")")
                                .foregroundColor(.green)
                        }
                    }
                }
                .frame(height: 200)
            } else {
                Text("No order book data available")
                    .foregroundColor(.gray)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }
    
    private var balancesView: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Balances")
                .font(.headline)
                .fontWeight(.bold)
            
            LazyColumn {
                ForEach(tradingClient.balances) { balance in
                    HStack {
                        Text(balance.asset)
                            .fontWeight(.bold)
                        Spacer()
                        VStack(alignment: .trailing) {
                            Text("\(balance.available, specifier: "%.8f")")
                            Text("(\(balance.reserved, specifier: "%.8f") reserved)")
                                .font(.caption)
                                .foregroundColor(.gray)
                        }
                    }
                }
            }
            .frame(height: 150)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }
    
    private var tradesView: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Recent Trades")
                .font(.headline)
                .fontWeight(.bold)
            
            LazyColumn {
                ForEach(tradingClient.trades.prefix(10)) { trade in
                    HStack {
                        Text(trade.side.uppercased())
                            .foregroundColor(trade.side == "buy" ? .green : .red)
                            .fontWeight(.bold)
                        
                        Text("\(trade.quantity, specifier: "%.6f") @ $\(trade.price, specifier: "%.2f")")
                        
                        Spacer()
                        
                        Text(trade.timestamp, style: .time)
                            .font(.caption)
                            .foregroundColor(.gray)
                    }
                }
            }
            .frame(height: 150)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(10)
        .shadow(radius: 2)
    }
}

// MARK: - App Entry Point

@main
struct CryptoExchangeApp: App {
    var body: some Scene {
        WindowGroup {
            TradingView()
        }
    }
}

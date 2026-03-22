package com.cryptoexchange.trading

import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.*
import org.json.JSONObject
import java.net.URI
import java.util.concurrent.TimeUnit
import tech.gusavila92.websocketclient.WebSocketClient

data class Balance(
    val asset: String,
    val available: Double,
    val reserved: Double
)

data class OrderBookEntry(
    val price: Double,
    val quantity: Double
)

data class Trade(
    val id: String,
    val pair: String,
    val price: Double,
    val quantity: Double,
    val side: String,
    val timestamp: Long
)

data class Order(
    val id: Long,
    val side: String,
    val orderType: String,
    val price: Double?,
    val quantity: Double,
    val pair: String,
    val status: String,
    val filledQuantity: Double,
    val remainingQuantity: Double
)

class MainActivity : ComponentActivity() {
    private lateinit var webSocketClient: WebSocketClient
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    
    // Trading state
    private val balances = mutableStateMapOf<String, Balance>()
    private val orderBooks = mutableStateMapOf<String, Pair<List<OrderBookEntry>, List<OrderBookEntry>>>()
    private val trades = mutableStateListOf<Trade>()
    private val orders = mutableStateMapOf<Long, Order>()
    private var nextOrderId = 1L
    
    // UI state
    private var selectedPair by mutableStateOf("BTC/USDT")
    private var orderSide by mutableStateOf("buy")
    private var orderType by mutableStateOf("market")
    private var orderPrice by mutableStateOf("50000")
    private var orderQuantity by mutableStateOf("0.001")
    private var isConnected by mutableStateOf(false)
    private var autoTrading by mutableStateOf(false)
    private var clientId by mutableStateOf("android_${System.currentTimeMillis()}")

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    TradingApp()
                }
            }
        }
        
        initializeWebSocket()
        connectWebSocket()
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
        webSocketClient.close()
    }

    private fun initializeWebSocket() {
        webSocketClient = object : WebSocketClient(URI("ws://127.0.0.1:8080/ws")) {
            override fun onOpen() {
                Log.d("WebSocket", "Connection opened")
                isConnected = true
                sendIdentify()
            }

            override fun onMessage(message: String?) {
                message?.let {
                    handleMessage(it)
                }
            }

            override fun onClose(code: Int, reason: String?, remote: Boolean) {
                Log.d("WebSocket", "Connection closed: $reason")
                isConnected = false
            }

            override fun onError(exception: Exception?) {
                Log.e("WebSocket", "Error: ${exception?.message}")
                isConnected = false
            }
        }
    }

    private fun connectWebSocket() {
        try {
            webSocketClient.connect()
        } catch (e: Exception) {
            Log.e("WebSocket", "Failed to connect: ${e.message}")
        }
    }

    private fun sendIdentify() {
        val message = JSONObject().apply {
            put("type", "Identify")
            put("client_id", clientId)
            put("platform", "Android")
        }
        webSocketClient.sendMessage(message.toString())
    }

    private fun handleMessage(message: String) {
        scope.launch {
            try {
                val json = JSONObject(message)
                val type = json.getString("type")
                
                when (type) {
                    "BalanceUpdate" -> handleBalanceUpdate(json)
                    "OrderBookUpdate" -> handleOrderBookUpdate(json)
                    "Trade" -> handleTrade(json)
                    "OrderUpdate" -> handleOrderUpdate(json)
                    "Response" -> Log.d("Server", json.getString("message"))
                }
            } catch (e: Exception) {
                Log.e("WebSocket", "Failed to handle message: ${e.message}")
            }
        }
    }

    private fun handleBalanceUpdate(json: JSONObject) {
        val asset = json.getString("asset")
        val available = json.getDouble("available") / 100_000_000
        val reserved = json.getDouble("reserved") / 100_000_000
        
        balances[asset] = Balance(asset, available, reserved)
        Log.d("Balance", "$asset: $available available, $reserved reserved")
    }

    private fun handleOrderBookUpdate(json: JSONObject) {
        val pair = json.getString("pair")
        val bidsArray = json.getJSONArray("bids")
        val asksArray = json.getJSONArray("asks")
        
        val bids = mutableListOf<OrderBookEntry>()
        val asks = mutableListOf<OrderBookEntry>()
        
        for (i in 0 until bidsArray.length()) {
            val bid = bidsArray.getJSONArray(i)
            bids.add(OrderBookEntry(
                bid.getDouble(0) / 100,
                bid.getDouble(1) / 1000
            ))
        }
        
        for (i in 0 until asksArray.length()) {
            val ask = asksArray.getJSONArray(i)
            asks.add(OrderBookEntry(
                ask.getDouble(0) / 100,
                ask.getDouble(1) / 1000
            ))
        }
        
        orderBooks[pair] = Pair(bids, asks)
        
        if (bids.isNotEmpty() && asks.isNotEmpty()) {
            Log.d("OrderBook", "$pair - Bid: $${bids[0].price}, Ask: $${asks[0].price}")
        }
    }

    private fun handleTrade(json: JSONObject) {
        val trade = Trade(
            id = json.getString("id"),
            pair = json.getString("pair"),
            price = json.getDouble("price") / 100,
            quantity = json.getDouble("quantity") / 1000,
            side = json.getString("side"),
            timestamp = json.getLong("timestamp")
        )
        
        trades.add(0, trade) // Add to front
        
        // Keep only last 50 trades
        while (trades.size > 50) {
            trades.removeAt(trades.size - 1)
        }
        
        Log.d("Trade", "${trade.side.uppercase()} ${trade.quantity} ${trade.pair} @ $${trade.price}")
    }

    private fun handleOrderUpdate(json: JSONObject) {
        val orderId = json.getLong("id")
        val status = json.getString("status")
        val filledQty = json.getDouble("filled_quantity") / 1000
        val remainingQty = json.getDouble("remaining_quantity") / 1000
        
        orders[orderId]?.let { order ->
            orders[orderId] = order.copy(
                status = status,
                filledQuantity = filledQty,
                remainingQuantity = remainingQty
            )
            Log.d("Order", "Order $orderId - Status: $status, Filled: $filledQty")
        }
    }

    private fun placeOrder() {
        val message = JSONObject().apply {
            put("type", "PlaceOrder")
            put("id", nextOrderId++)
            put("side", orderSide)
            put("order_type", orderType)
            if (orderType == "limit") {
                put("price", (orderPrice.toDouble() * 100).toLong())
            }
            put("quantity", (orderQuantity.toDouble() * 1000).toLong())
            put("pair", selectedPair)
        }
        
        webSocketClient.sendMessage(message.toString())
        Log.d("Order", "Placed $orderSide $orderType order: $orderQuantity $selectedPair")
    }

    private fun cancelOrder(orderId: Long) {
        val message = JSONObject().apply {
            put("type", "CancelOrder")
            put("id", orderId)
        }
        webSocketClient.sendMessage(message.toString())
        Log.d("Order", "Cancelled order $orderId")
    }

    private fun placeRandomOrder() {
        val sides = listOf("buy", "sell")
        val types = listOf("market", "limit")
        
        orderSide = sides.random()
        orderType = types.random()
        
        if (orderType == "limit") {
            val basePrice = if (selectedPair == "BTC/USDT") 50000.0 else 3000.0
            val variation = (Math.random() * 2000 - 1000)
            orderPrice = String.format("%.2f", basePrice + variation)
        }
        
        orderQuantity = String.format("%.6f", Math.random() * 0.01 + 0.001)
        placeOrder()
    }

    private fun startAutoTrading() {
        scope.launch {
            while (autoTrading && isConnected) {
                placeRandomOrder()
                delay(5000) // 5 seconds between trades
            }
        }
    }

    @Composable
    fun TradingApp() {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp)
        ) {
            // Header
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "📱 Crypto Exchange",
                    fontSize = 20.sp,
                    fontWeight = FontWeight.Bold
                )
                
                Row {
                    Text(
                        text = if (isConnected) "🟢 Connected" else "🔴 Disconnected",
                        color = if (isConnected) Color.Green else Color.Red
                    )
                    
                    Spacer(modifier = Modifier.width(8.dp))
                    
                    Switch(
                        checked = autoTrading,
                        onCheckedChange = { 
                            autoTrading = it
                            if (it) startAutoTrading()
                        }
                    )
                    Text("Auto Trade")
                }
            }
            
            Spacer(modifier = Modifier.height(16.dp))
            
            // Order Form
            Card(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(16.dp)
                ) {
                    Text(
                        text = "Place Order",
                        fontSize = 18.sp,
                        fontWeight = FontWeight.Bold
                    )
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    // Trading Pair
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text("Pair:")
                        OutlinedTextField(
                            value = selectedPair,
                            onValueChange = { selectedPair = it },
                            modifier = Modifier.width(120.dp)
                        )
                    }
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    // Order Side
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text("Side:")
                        Row {
                            RadioButton(
                                selected = orderSide == "buy",
                                onClick = { orderSide = "buy" }
                            )
                            Text("Buy")
                            Spacer(modifier = Modifier.width(16.dp))
                            RadioButton(
                                selected = orderSide == "sell",
                                onClick = { orderSide = "sell" }
                            )
                            Text("Sell")
                        }
                    }
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    // Order Type
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text("Type:")
                        Row {
                            RadioButton(
                                selected = orderType == "market",
                                onClick = { orderType = "market" }
                            )
                            Text("Market")
                            Spacer(modifier = Modifier.width(16.dp))
                            RadioButton(
                                selected = orderType == "limit",
                                onClick = { orderType = "limit" }
                            )
                            Text("Limit")
                        }
                    }
                    
                    Spacer(modifier = Modifier.height(8.dp))
                    
                    // Price (for limit orders)
                    if (orderType == "limit") {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text("Price:")
                            OutlinedTextField(
                                value = orderPrice,
                                onValueChange = { orderPrice = it },
                                modifier = Modifier.width(120.dp)
                            )
                        }
                        
                        Spacer(modifier = Modifier.height(8.dp))
                    }
                    
                    // Quantity
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text("Quantity:")
                        OutlinedTextField(
                            value = orderQuantity,
                            onValueChange = { orderQuantity = it },
                            modifier = Modifier.width(120.dp)
                        )
                    }
                    
                    Spacer(modifier = Modifier.height(16.dp))
                    
                    // Buttons
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceEvenly
                    ) {
                        Button(
                            onClick = { placeOrder() },
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (orderSide == "buy") Color.Green else Color.Red
                            )
                        ) {
                            Text("Place ${orderSide.uppercase()} Order")
                        }
                        
                        Button(
                            onClick = { 
                                orderQuantity = "0.001"
                                orderPrice = "50000"
                            }
                        ) {
                            Text("Clear")
                        }
                    }
                }
            }
            
            Spacer(modifier = Modifier.height(16.dp))
            
            // Order Book
            Card(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(16.dp)
                ) {
                    Text(
                        text = "Order Book",
                        fontSize = 18.sp,
                        fontWeight = FontWeight.Bold
                    )
                    
                    orderBooks[selectedPair]?.let { (bids, asks) ->
                        LazyColumn(
                            modifier = Modifier.height(200.dp)
                        ) {
                            // Asks (red)
                            items(asks.take(5).reversed()) { ask ->
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text(
                                        text = "$${ask.price}",
                                        color = Color.Red
                                    )
                                    Text(
                                        text = "${ask.quantity}",
                                        color = Color.Red
                                    )
                                }
                            }
                            
                            // Spread
                            if (bids.isNotEmpty() && asks.isNotEmpty()) {
                                val spread = asks[0].price - bids[0].price
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.Center
                                ) {
                                    Text(
                                        text = "Spread: $${String.format("%.2f", spread)}",
                                        color = Color.Gray
                                    )
                                }
                            }
                            
                            // Bids (green)
                            items(bids.take(5)) { bid ->
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text(
                                        text = "$${bid.price}",
                                        color = Color.Green
                                    )
                                    Text(
                                        text = "${bid.quantity}",
                                        color = Color.Green
                                    )
                                }
                            }
                        }
                    } ?: run {
                        Text("No order book data available")
                    }
                }
            }
            
            Spacer(modifier = Modifier.height(16.dp))
            
            // Balances
            Card(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(16.dp)
                ) {
                    Text(
                        text = "Balances",
                        fontSize = 18.sp,
                        fontWeight = FontWeight.Bold
                    )
                    
                    LazyColumn(
                        modifier = Modifier.height(150.dp)
                    ) {
                        items(balances.values.toList()) { balance ->
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text(
                                    text = balance.asset,
                                    fontWeight = FontWeight.Bold
                                )
                                Column {
                                    Text("${String.format("%.8f", balance.available)}")
                                    Text(
                                        text = "(${String.format("%.8f", balance.reserved)} reserved)",
                                        fontSize = 12.sp,
                                        color = Color.Gray
                                    )
                                }
                            }
                        }
                    }
                }
            }
            
            Spacer(modifier = Modifier.height(16.dp))
            
            // Recent Trades
            Card(
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(
                    modifier = Modifier.padding(16.dp)
                ) {
                    Text(
                        text = "Recent Trades",
                        fontSize = 18.sp,
                        fontWeight = FontWeight.Bold
                    )
                    
                    LazyColumn(
                        modifier = Modifier.height(150.dp)
                    ) {
                        items(trades.take(10)) { trade ->
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text(
                                    text = trade.side.uppercase(),
                                    color = if (trade.side == "buy") Color.Green else Color.Red,
                                    fontWeight = FontWeight.Bold
                                )
                                Text("${trade.quantity} @ $${trade.price}")
                                Text(
                                    text = java.text.SimpleDateFormat("HH:mm:ss").format(java.util.Date(trade.timestamp / 1000000)),
                                    fontSize = 12.sp,
                                    color = Color.Gray
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

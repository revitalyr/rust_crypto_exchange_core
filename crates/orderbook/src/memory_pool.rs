//! Memory pool for efficient allocation of order book nodes.

use crypto_exchange_common::{order::Order, ExchangeError, ExchangeResult};
use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Node in the order book
#[derive(Debug)]
pub struct OrderNode {
    /// The order
    pub order: Order,
    /// Next node at the same price level
    pub next: Option<*mut OrderNode>,
    /// Previous node at the same price level
    pub prev: Option<*mut OrderNode>,
    /// Parent price level
    pub price_level: Option<*mut PriceLevelNode>,
}

/// Price level node in the order book
#[derive(Debug)]
pub struct PriceLevelNode {
    /// Price value
    pub price: u64,
    /// Total quantity at this price
    pub total_quantity: u64,
    /// Number of orders at this price
    pub order_count: u64,
    /// Head of order list at this price
    pub head: Option<*mut OrderNode>,
    /// Tail of order list at this price
    pub tail: Option<*mut OrderNode>,
    /// Next price level
    pub next: Option<*mut PriceLevelNode>,
    /// Previous price level
    pub prev: Option<*mut PriceLevelNode>,
}

impl PriceLevelNode {
    /// Creates a new price level node
    pub fn new(price: u64) -> Self {
        Self {
            price,
            total_quantity: 0,
            order_count: 0,
            head: None,
            tail: None,
            next: None,
            prev: None,
        }
    }

    /// Adds an order to this price level
    pub fn add_order(&mut self, node: *mut OrderNode) {
        unsafe {
            (*node).price_level = Some(self as *mut PriceLevelNode);
            
            match self.tail {
                None => {
                    // First order at this price
                    self.head = Some(node);
                    self.tail = Some(node);
                    (*node).prev = None;
                    (*node).next = None;
                }
                Some(tail) => {
                    // Add to end of list
                    (*tail).next = Some(node);
                    (*node).prev = Some(tail);
                    (*node).next = None;
                    self.tail = Some(node);
                }
            }
            
            self.total_quantity += (*node).order.remaining_quantity();
            self.order_count += 1;
        }
    }

    /// Removes an order from this price level
    pub fn remove_order(&mut self, node: *mut OrderNode) {
        unsafe {
            let order = &(*node).order;
            self.total_quantity -= order.remaining_quantity();
            self.order_count -= 1;

            let prev = (*node).prev;
            let next = (*node).next;

            match prev {
                None => {
                    // Node was head
                    self.head = next;
                }
                Some(prev_node) => {
                    (*prev_node).next = next;
                }
            }

            match next {
                None => {
                    // Node was tail
                    self.tail = prev;
                }
                Some(next_node) => {
                    (*next_node).prev = prev;
                }
            }

            (*node).prev = None;
            (*node).next = None;
            (*node).price_level = None;
        }
    }

    /// Checks if this price level is empty
    pub fn is_empty(&self) -> bool {
        self.order_count == 0
    }
}

/// Memory pool for order nodes
pub struct OrderNodePool {
    /// Pool of available nodes
    pool: SegQueue<*mut OrderNode>,
    /// Number of allocated nodes
    allocated: AtomicUsize,
    /// Number of nodes in pool
    pooled: AtomicUsize,
    /// Maximum pool size
    max_pool_size: usize,
}

impl OrderNodePool {
    /// Creates a new order node pool
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pool: SegQueue::new(),
            allocated: AtomicUsize::new(0),
            pooled: AtomicUsize::new(0),
            max_pool_size,
        }
    }

    /// Allocates a new order node
    pub fn allocate(&self, order: Order) -> *mut OrderNode {
        // Try to reuse from pool first
        if let Some(ptr) = self.pool.pop() {
            self.pooled.fetch_sub(1, Ordering::Relaxed);
            unsafe {
                // Reinitialize the node
                (*ptr).order = order;
                (*ptr).next = None;
                (*ptr).prev = None;
                (*ptr).price_level = None;
                return ptr;
            }
        }

        // Allocate new node
        let node = OrderNode {
            order,
            next: None,
            prev: None,
            price_level: None,
        };
        let ptr = Box::into_raw(Box::new(node));
        self.allocated.fetch_add(1, Ordering::Relaxed);
        ptr
    }

    /// Deallocates an order node
    pub fn deallocate(&self, ptr: *mut OrderNode) {
        if self.pooled.load(Ordering::Relaxed) < self.max_pool_size {
            // Return to pool
            self.pool.push(ptr);
            self.pooled.fetch_add(1, Ordering::Relaxed);
        } else {
            // Pool is full, deallocate
            unsafe {
                drop(Box::from_raw(ptr));
            }
            self.allocated.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Returns pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            allocated: self.allocated.load(Ordering::Relaxed),
            pooled: self.pooled.load(Ordering::Relaxed),
            max_pool_size: self.max_pool_size,
        }
    }
}

/// Memory pool for price level nodes
pub struct PriceLevelNodePool {
    /// Pool of available nodes
    pool: SegQueue<*mut PriceLevelNode>,
    /// Number of allocated nodes
    allocated: AtomicUsize,
    /// Number of nodes in pool
    pooled: AtomicUsize,
    /// Maximum pool size
    max_pool_size: usize,
}

impl PriceLevelNodePool {
    /// Creates a new price level node pool
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            pool: SegQueue::new(),
            allocated: AtomicUsize::new(0),
            pooled: AtomicUsize::new(0),
            max_pool_size,
        }
    }

    /// Allocates a new price level node
    pub fn allocate(&self, price: u64) -> *mut PriceLevelNode {
        // Try to reuse from pool first
        if let Some(ptr) = self.pool.pop() {
            self.pooled.fetch_sub(1, Ordering::Relaxed);
            unsafe {
                // Reinitialize the node
                (*ptr) = PriceLevelNode::new(price);
                return ptr;
            }
        }

        // Allocate new node
        let node = PriceLevelNode::new(price);
        let ptr = Box::into_raw(Box::new(node));
        self.allocated.fetch_add(1, Ordering::Relaxed);
        ptr
    }

    /// Deallocates a price level node
    pub fn deallocate(&self, ptr: *mut PriceLevelNode) {
        if self.pooled.load(Ordering::Relaxed) < self.max_pool_size {
            // Return to pool
            self.pool.push(ptr);
            self.pooled.fetch_add(1, Ordering::Relaxed);
        } else {
            // Pool is full, deallocate
            unsafe {
                drop(Box::from_raw(ptr));
            }
            self.allocated.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Returns pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            allocated: self.allocated.load(Ordering::Relaxed),
            pooled: self.pooled.load(Ordering::Relaxed),
            max_pool_size: self.max_pool_size,
        }
    }
}

/// Combined memory pool for both order and price level nodes
pub struct MemoryPool {
    /// Order node pool
    order_pool: OrderNodePool,
    /// Price level node pool
    price_level_pool: PriceLevelNodePool,
    /// Next order ID
    next_order_id: AtomicU64,
}

impl MemoryPool {
    /// Creates a new memory pool
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            order_pool: OrderNodePool::new(max_pool_size),
            price_level_pool: PriceLevelNodePool::new(max_pool_size / 10), // Fewer price levels
            next_order_id: AtomicU64::new(1),
        }
    }

    /// Allocates a new order node
    pub fn allocate_order(&self, order: Order) -> *mut OrderNode {
        self.order_pool.allocate(order)
    }

    /// Deallocates an order node
    pub fn deallocate_order(&self, ptr: *mut OrderNode) {
        self.order_pool.deallocate(ptr);
    }

    /// Allocates a new price level node
    pub fn allocate_price_level(&self, price: u64) -> *mut PriceLevelNode {
        self.price_level_pool.allocate(price)
    }

    /// Deallocates a price level node
    pub fn deallocate_price_level(&self, ptr: *mut PriceLevelNode) {
        self.price_level_pool.deallocate(ptr);
    }

    /// Gets the next order ID
    pub fn next_order_id(&self) -> u64 {
        self.next_order_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns pool statistics
    pub fn stats(&self) -> MemoryPoolStats {
        MemoryPoolStats {
            order_pool: self.order_pool.stats(),
            price_level_pool: self.price_level_pool.stats(),
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of allocated nodes
    pub allocated: usize,
    /// Number of nodes in pool
    pub pooled: usize,
    /// Maximum pool size
    pub max_pool_size: usize,
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    /// Order pool statistics
    pub order_pool: PoolStats,
    /// Price level pool statistics
    pub price_level_pool: PoolStats,
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new(10000)
    }
}

unsafe impl Send for OrderNode {}
unsafe impl Sync for OrderNode {}

unsafe impl Send for PriceLevelNode {}
unsafe impl Sync for PriceLevelNode {}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_exchange_common::{order::OrderSide, price::Price};

    #[test]
    fn test_memory_pool_allocation() {
        let pool = MemoryPool::new(100);
        
        let order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            crypto_exchange_common::order::OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            crypto_exchange_common::order::TimeInForce::GTC,
            1234567890,
        );

        let ptr = pool.allocate_order(order.clone());
        assert!(!ptr.is_null());

        // Verify the order was stored correctly
        unsafe {
            assert_eq!((*ptr).order.id, order.id);
            assert_eq!((*ptr).order.user_id, order.user_id);
        }

        pool.deallocate_order(ptr);
    }

    #[test]
    fn test_price_level_node_operations() {
        let pool = MemoryPool::new(100);
        let mut level = PriceLevelNode::new(50000);

        // Create some test orders
        let order1 = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            crypto_exchange_common::order::OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            crypto_exchange_common::order::TimeInForce::GTC,
            1234567890,
        );

        let order2 = Order::new(
            2,
            101,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            crypto_exchange_common::order::OrderType::Limit,
            Some(Price::new(50000)),
            500,
            crypto_exchange_common::order::TimeInForce::GTC,
            1234567891,
        );

        let node1 = pool.allocate_order(order1);
        let node2 = pool.allocate_order(order2);

        // Add orders to level
        level.add_order(node1);
        assert_eq!(level.order_count, 1);
        assert_eq!(level.total_quantity, 1000);

        level.add_order(node2);
        assert_eq!(level.order_count, 2);
        assert_eq!(level.total_quantity, 1500);

        // Remove first order
        level.remove_order(node1);
        assert_eq!(level.order_count, 1);
        assert_eq!(level.total_quantity, 500);

        // Clean up
        pool.deallocate_order(node1);
        pool.deallocate_order(node2);
    }

    #[test]
    fn test_pool_reuse() {
        let pool = MemoryPool::new(10);
        
        let order = Order::new(
            1,
            100,
            "BTC/USDT".to_string(),
            OrderSide::Buy,
            crypto_exchange_common::order::OrderType::Limit,
            Some(Price::new(50000)),
            1000,
            crypto_exchange_common::order::TimeInForce::GTC,
            1234567890,
        );

        let ptr1 = pool.allocate_order(order.clone());
        pool.deallocate_order(ptr1);

        let ptr2 = pool.allocate_order(order);
        
        // Should reuse the same memory (though we can't guarantee this)
        assert!(!ptr2.is_null());
        
        pool.deallocate_order(ptr2);
    }
}

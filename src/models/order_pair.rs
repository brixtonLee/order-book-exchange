use crate::Order;

pub struct OrderPair<'a> {
    incoming_order: &'a mut Order,
    resting_order: &'a mut Order,
}

impl<'a> OrderPair<'a> {
    /// Create a new OrderPair
    pub fn new(incoming_order: &'a mut Order, resting_order: &'a mut Order) -> Self {
        Self {
            incoming_order,
            resting_order,
        }
    }

    /// Get immutable reference to incoming order
    pub fn incoming_order(&self) -> &Order {
        self.incoming_order
    }

    /// Get mutable reference to incoming order
    pub fn incoming_order_mut(&mut self) -> &mut Order {
        self.incoming_order
    }

    /// Get immutable reference to resting order
    pub fn resting_order(&self) -> &Order {
        self.resting_order
    }

    /// Get mutable reference to resting order
    pub fn resting_order_mut(&mut self) -> &mut Order {
        self.resting_order
    }
}
use bytes::{Buf, BufMut, BytesMut};
use std::io::{self, Error, ErrorKind};
use uuid::Uuid;
use chrono::DateTime;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::models::{Order, OrderSide, OrderType, TimeInForce, OrderStatus};
use crate::models::order::SelfTradePreventionMode;

/// Price multiplier for fixed-point encoding
/// Price of 100.12345678 becomes 10012345678i64
const PRICE_SCALE: i64 = 100_000_000; // 10^8

/// Message types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    NewOrder = 1,
    CancelOrder = 2,
    ModifyOrder = 3,
    ExecutionReport = 4,
    OrderBookSnapshot = 5,
    Trade = 6,
    Heartbeat = 255,
}

impl TryFrom<u8> for MessageType {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::NewOrder),
            2 => Ok(MessageType::CancelOrder),
            3 => Ok(MessageType::ModifyOrder),
            4 => Ok(MessageType::ExecutionReport),
            5 => Ok(MessageType::OrderBookSnapshot),
            6 => Ok(MessageType::Trade),
            255 => Ok(MessageType::Heartbeat),
            _ => Err(Error::new(ErrorKind::InvalidData, "Unknown message type")),
        }
    }
}

/// Binary order message (52 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryOrderMessage {
    pub msg_type: u8,           // 1 byte
    pub side: u8,               // 1 byte: 0=Buy, 1=Sell
    pub order_type: u8,         // 1 byte: 0=Limit, 1=Market
    pub time_in_force: u8,      // 1 byte: 0=GTC, 1=IOC, 2=FOK, etc.
    pub price: i64,             // 8 bytes: Fixed-point (price × 10^8)
    pub quantity: i64,          // 8 bytes: Fixed-point (qty × 10^8)
    pub symbol: [u8; 8],        // 8 bytes: Null-padded ASCII
    pub order_id: [u8; 16],     // 16 bytes: UUID bytes
    pub timestamp_ns: u64,      // 8 bytes: Nanoseconds since epoch
}

impl BinaryOrderMessage {
    pub const SIZE: usize = 52;

    /// Encode to bytes (zero-copy into buffer)
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(self.msg_type);
        buf.put_u8(self.side);
        buf.put_u8(self.order_type);
        buf.put_u8(self.time_in_force);
        buf.put_i64(self.price);
        buf.put_i64(self.quantity);
        buf.put_slice(&self.symbol);
        buf.put_slice(&self.order_id);
        buf.put_u64(self.timestamp_ns);
    }

    /// Decode from bytes (zero-copy from buffer)
    pub fn decode(buf: &mut impl Buf) -> io::Result<Self> {
        if buf.remaining() < Self::SIZE {
            return Err(Error::new(ErrorKind::UnexpectedEof, "Incomplete message"));
        }

        let msg_type = buf.get_u8();
        let side = buf.get_u8();
        let order_type = buf.get_u8();
        let time_in_force = buf.get_u8();
        let price = buf.get_i64();
        let quantity = buf.get_i64();

        let mut symbol = [0u8; 8];
        buf.copy_to_slice(&mut symbol);

        let mut order_id = [0u8; 16];
        buf.copy_to_slice(&mut order_id);

        let timestamp_ns = buf.get_u64();

        Ok(Self {
            msg_type,
            side,
            order_type,
            time_in_force,
            price,
            quantity,
            symbol,
            order_id,
            timestamp_ns,
        })
    }

    /// Convert to domain Order type
    pub fn to_order(&self) -> Result<Order, io::Error> {
        let symbol = String::from_utf8_lossy(&self.symbol)
            .trim_end_matches('\0')
            .to_string();

        let price = if self.order_type == 0 {
            Some(Decimal::new(self.price, 8))
        } else {
            None
        };

        let timestamp = DateTime::from_timestamp_nanos(self.timestamp_ns as i64);

        Ok(Order {
            id: Uuid::from_bytes(self.order_id),
            symbol,
            side: if self.side == 0 { OrderSide::Buy } else { OrderSide::Sell },
            order_type: if self.order_type == 0 { OrderType::Limit } else { OrderType::Market },
            price,
            quantity: Decimal::new(self.quantity, 8),
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: "binary".to_string(), // Default for binary protocol
            timestamp,
            time_in_force: match self.time_in_force {
                0 => TimeInForce::GTC,
                1 => TimeInForce::IOC,
                2 => TimeInForce::FOK,
                3 => TimeInForce::GTD,
                4 => TimeInForce::DAY,
                _ => TimeInForce::GTC,
            },
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            iceberg: None,
        })
    }

    /// Create from domain Order type
    pub fn from_order(order: &Order) -> Self {
        let mut symbol = [0u8; 8];
        let symbol_bytes = order.symbol.as_bytes();
        let copy_len = symbol_bytes.len().min(8);
        symbol[..copy_len].copy_from_slice(&symbol_bytes[..copy_len]);

        let price = order.price
            .map(|p| {
                let scaled = p * Decimal::new(PRICE_SCALE, 0);
                scaled.to_i64().unwrap_or(0)
            })
            .unwrap_or(0);

        let quantity = {
            let scaled = order.quantity * Decimal::new(PRICE_SCALE, 0);
            scaled.to_i64().unwrap_or(0)
        };

        let timestamp_ns = order.timestamp
            .timestamp_nanos_opt()
            .unwrap_or(0) as u64;

        Self {
            msg_type: MessageType::NewOrder as u8,
            side: if order.side == OrderSide::Buy { 0 } else { 1 },
            order_type: if order.order_type == OrderType::Limit { 0 } else { 1 },
            time_in_force: match order.time_in_force {
                TimeInForce::GTC => 0,
                TimeInForce::IOC => 1,
                TimeInForce::FOK => 2,
                TimeInForce::GTD => 3,
                TimeInForce::DAY => 4,
            },
            price,
            quantity,
            symbol,
            order_id: *order.id.as_bytes(),
            timestamp_ns,
        }
    }
}

/// Framed message with length prefix
pub struct FramedCodec;

impl FramedCodec {
    /// Encode a message with 2-byte length prefix
    pub fn encode_framed(msg: &BinaryOrderMessage, buf: &mut BytesMut) {
        buf.put_u16(BinaryOrderMessage::SIZE as u16);
        msg.encode(buf);
    }

    /// Decode a framed message
    pub fn decode_framed(buf: &mut impl Buf) -> io::Result<Option<BinaryOrderMessage>> {
        if buf.remaining() < 2 {
            return Ok(None); // Need more data
        }

        // Peek at length without consuming
        let peek_buf = buf.chunk();
        if peek_buf.len() < 2 {
            return Ok(None);
        }

        let len = u16::from_be_bytes([peek_buf[0], peek_buf[1]]) as usize;

        if buf.remaining() < 2 + len {
            return Ok(None); // Need more data
        }

        // Consume the length prefix
        buf.advance(2);

        BinaryOrderMessage::decode(buf).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn create_test_order() -> Order {
        Order {
            id: Uuid::new_v4(),
            symbol: "BTCUSD".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(dec!(50000.12345678)),
            quantity: dec!(1.5),
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: "test_user".to_string(),
            timestamp: Utc::now(),
            time_in_force: TimeInForce::GTC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            iceberg: None,
        }
    }

    #[test]
    fn test_binary_message_size() {
        assert_eq!(BinaryOrderMessage::SIZE, 52);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let order = create_test_order();
        let binary_msg = BinaryOrderMessage::from_order(&order);

        let mut buf = BytesMut::with_capacity(64);
        binary_msg.encode(&mut buf);

        assert_eq!(buf.len(), BinaryOrderMessage::SIZE);

        let decoded = BinaryOrderMessage::decode(&mut buf).unwrap();

        // Copy values to avoid packed field reference errors
        let orig_price = binary_msg.price;
        let orig_quantity = binary_msg.quantity;
        let decoded_price = decoded.price;
        let decoded_quantity = decoded.quantity;

        assert_eq!(decoded.side, binary_msg.side);
        assert_eq!(decoded.order_type, binary_msg.order_type);
        assert_eq!(decoded_price, orig_price);
        assert_eq!(decoded_quantity, orig_quantity);
    }

    #[test]
    fn test_framed_codec() {
        let order = create_test_order();
        let binary_msg = BinaryOrderMessage::from_order(&order);

        let mut buf = BytesMut::with_capacity(128);
        FramedCodec::encode_framed(&binary_msg, &mut buf);

        assert_eq!(buf.len(), 2 + BinaryOrderMessage::SIZE);

        let decoded = FramedCodec::decode_framed(&mut buf).unwrap();
        assert!(decoded.is_some());

        let decoded_msg = decoded.unwrap();
        assert_eq!(decoded_msg.side, binary_msg.side);
    }

    #[test]
    fn test_symbol_truncation() {
        let mut order = create_test_order();
        order.symbol = "VERYLONGSYMBOL".to_string();

        let binary_msg = BinaryOrderMessage::from_order(&order);

        // Symbol should be truncated to 8 bytes
        let symbol_str = String::from_utf8_lossy(&binary_msg.symbol);
        assert!(symbol_str.len() <= 8);
    }

    #[test]
    fn test_fixed_point_precision() {
        let order = create_test_order();
        let binary_msg = BinaryOrderMessage::from_order(&order);

        // Convert back and check precision
        let reconstructed = binary_msg.to_order().unwrap();

        // Should preserve 8 decimal places
        if let (Some(original_price), Some(reconstructed_price)) = (order.price, reconstructed.price) {
            let diff = (original_price - reconstructed_price).abs();
            assert!(diff < dec!(0.00000001));
        }
    }

    #[test]
    fn test_incomplete_message() {
        let mut buf = BytesMut::with_capacity(10);
        buf.put_u8(1); // msg_type
        buf.put_u8(0); // side
        // Only 2 bytes, not enough

        let result = BinaryOrderMessage::decode(&mut buf);
        assert!(result.is_err());
    }
}

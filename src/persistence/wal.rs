use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use bincode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::models::{Order, Trade};

/// Events that get persisted to WAL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalEvent {
    /// New order submitted
    OrderSubmitted {
        sequence: u64,
        timestamp_ns: u64,
        order: Order,
    },

    /// Order was cancelled
    OrderCancelled {
        sequence: u64,
        timestamp_ns: u64,
        order_id: Uuid,
        symbol: String,
    },

    /// Trade executed
    TradeExecuted {
        sequence: u64,
        timestamp_ns: u64,
        trade: Trade,
    },

    /// Order modified
    OrderModified {
        sequence: u64,
        timestamp_ns: u64,
        order_id: Uuid,
        new_quantity: Option<Decimal>,
        new_price: Option<Decimal>,
    },

    /// Checkpoint marker (state was snapshotted)
    Checkpoint {
        sequence: u64,
        timestamp_ns: u64,
        checkpoint_path: String,
    },
}

/// Write-Ahead Log for durability
pub struct WriteAheadLog {
    /// Current WAL file
    file: BufWriter<File>,

    /// Current sequence number
    sequence: u64,

    /// Path to WAL directory
    wal_dir: PathBuf,

    /// Current WAL file index
    file_index: u64,

    /// Max size before rotation (default 100MB)
    max_file_size: u64,

    /// Current file size
    current_size: u64,

    /// Sync mode
    sync_mode: SyncMode,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncMode {
    /// fsync after every write (safest, slowest)
    EveryWrite,
    /// fsync every N writes
    Batched(u32),
    /// Let OS handle syncing (fastest, least safe)
    None,
}

impl WriteAheadLog {
    pub fn open(wal_dir: impl AsRef<Path>, sync_mode: SyncMode) -> io::Result<Self> {
        let wal_dir = wal_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&wal_dir)?;

        // Find the latest WAL file or create new one
        let (file_index, sequence) = Self::find_latest_wal(&wal_dir)?;

        let wal_path = wal_dir.join(format!("wal_{:08}.log", file_index));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)?;

        let current_size = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

        Ok(Self {
            file: BufWriter::new(file),
            sequence,
            wal_dir,
            file_index,
            max_file_size: 100 * 1024 * 1024, // 100MB
            current_size,
            sync_mode,
        })
    }

    /// Append an event to the WAL
    pub fn append(&mut self, event: WalEvent) -> io::Result<u64> {
        self.sequence += 1;
        let seq = self.sequence;

        // Serialize event with length prefix
        let encoded = bincode::serialize(&event)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write length prefix (4 bytes) + data
        let len = encoded.len() as u32;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&encoded)?;

        self.current_size += 4 + encoded.len() as u64;

        // Handle sync mode
        match self.sync_mode {
            SyncMode::EveryWrite => {
                self.file.flush()?;
                self.file.get_ref().sync_data()?;
            }
            SyncMode::Batched(n) if seq % n as u64 == 0 => {
                self.file.flush()?;
                self.file.get_ref().sync_data()?;
            }
            _ => {}
        }

        // Rotate if needed
        if self.current_size >= self.max_file_size {
            self.rotate()?;
        }

        Ok(seq)
    }

    /// Rotate to a new WAL file
    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_data()?;

        self.file_index += 1;
        let new_path = self.wal_dir.join(format!("wal_{:08}.log", self.file_index));

        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&new_path)?;

        self.file = BufWriter::new(new_file);
        self.current_size = 0;

        Ok(())
    }

    /// Replay all events from WAL files
    pub fn replay<F>(&self, mut handler: F) -> io::Result<u64>
    where
        F: FnMut(WalEvent) -> io::Result<()>,
    {
        let mut count = 0;

        // Get all WAL files sorted by index
        let mut wal_files: Vec<_> = std::fs::read_dir(&self.wal_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect();

        wal_files.sort();

        for wal_path in wal_files {
            let file = File::open(&wal_path)?;
            let mut reader = BufReader::new(file);

            loop {
                // Read length prefix
                let mut len_buf = [0u8; 4];
                match reader.read_exact(&mut len_buf) {
                    Ok(_) => {}
                    Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                }

                let len = u32::from_le_bytes(len_buf) as usize;

                // Read event data
                let mut data = vec![0u8; len];
                reader.read_exact(&mut data)?;

                // Deserialize
                let event: WalEvent = bincode::deserialize(&data)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                handler(event)?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Force sync to disk
    pub fn sync(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_data()
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }

    fn find_latest_wal(wal_dir: &Path) -> io::Result<(u64, u64)> {
        // Find highest file index
        let mut max_index = 0u64;
        let mut max_sequence = 0u64;

        if let Ok(entries) = std::fs::read_dir(wal_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("wal_") && name.ends_with(".log") {
                        if let Ok(idx) = name[4..12].parse::<u64>() {
                            if idx > max_index {
                                max_index = idx;
                            }
                        }
                    }
                }
            }

            // Scan the latest file to find max sequence
            if max_index > 0 {
                let latest_path = wal_dir.join(format!("wal_{:08}.log", max_index));
                if latest_path.exists() {
                    if let Ok(file) = File::open(&latest_path) {
                        let mut reader = BufReader::new(file);
                        loop {
                            let mut len_buf = [0u8; 4];
                            match reader.read_exact(&mut len_buf) {
                                Ok(_) => {}
                                Err(_) => break,
                            }

                            let len = u32::from_le_bytes(len_buf) as usize;
                            let mut data = vec![0u8; len];
                            if reader.read_exact(&mut data).is_err() {
                                break;
                            }

                            if let Ok(event) = bincode::deserialize::<WalEvent>(&data) {
                                let seq = match event {
                                    WalEvent::OrderSubmitted { sequence, .. } => sequence,
                                    WalEvent::OrderCancelled { sequence, .. } => sequence,
                                    WalEvent::TradeExecuted { sequence, .. } => sequence,
                                    WalEvent::OrderModified { sequence, .. } => sequence,
                                    WalEvent::Checkpoint { sequence, .. } => sequence,
                                };
                                if seq > max_sequence {
                                    max_sequence = seq;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((max_index, max_sequence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use chrono::Utc;
    use rust_decimal_macros::dec;
    use crate::models::{OrderSide, OrderType, OrderStatus, TimeInForce};
    use crate::models::stp::SelfTradePreventionMode;

    fn create_test_order() -> Order {
        Order {
            id: Uuid::new_v4(),
            symbol: "TEST".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(dec!(100)),
            quantity: dec!(10),
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::New,
            user_id: "test".to_string(),
            timestamp: Utc::now(),
            time_in_force: TimeInForce::GTC,
            stp_mode: SelfTradePreventionMode::None,
            post_only: false,
            expire_time: None,
            iceberg: None,
        }
    }

    #[test]
    fn test_wal_create() {
        let temp_dir = TempDir::new().unwrap();
        let wal = WriteAheadLog::open(temp_dir.path(), SyncMode::None);
        assert!(wal.is_ok());
    }

    #[test]
    fn test_wal_append_and_replay() {
        let temp_dir = TempDir::new().unwrap();
        let mut wal = WriteAheadLog::open(temp_dir.path(), SyncMode::None).unwrap();

        let order = create_test_order();
        let event = WalEvent::OrderSubmitted {
            sequence: 1,
            timestamp_ns: Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            order: order.clone(),
        };

        let seq = wal.append(event).unwrap();
        assert_eq!(seq, 1);

        wal.sync().unwrap();

        // Replay
        let mut count = 0;
        wal.replay(|_event| {
            count += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_wal_multiple_events() {
        let temp_dir = TempDir::new().unwrap();
        let mut wal = WriteAheadLog::open(temp_dir.path(), SyncMode::None).unwrap();

        for i in 1..=5 {
            let order = create_test_order();
            let event = WalEvent::OrderSubmitted {
                sequence: i,
                timestamp_ns: Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                order,
            };
            wal.append(event).unwrap();
        }

        wal.sync().unwrap();

        let mut count = 0;
        wal.replay(|_event| {
            count += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(count, 5);
    }
}

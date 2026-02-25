// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use crate::storage::{
    DbFrame,
    FrameDatabase,
};

pub struct SqliteFrameDatabase {
    // Fields for the database connection and configuration
    connection: rusqlite::Connection,
}

impl SqliteFrameDatabase {
    pub fn new() -> Self {
        // Initialize the database connection and configuration
        let connection = rusqlite::Connection::open_in_memory().unwrap();

        let query = "
            CREATE TABLE IF NOT EXISTS frames (
              id            INTEGER PRIMARY KEY,
              timestamp     INTEGER NOT NULL,
              speed         INTEGER,
              rssi          INTEGER,
              channel       INTEGER,
              home_id       INTEGER NOT NULL,
              src_node_id   INTEGER,
              dst_node_id   INTEGER,
              payload_raw   BLOB NOT NULL
            );
            CREATE INDEX idx_frames_timestamp ON frames (timestamp);
        ";
        connection.execute_batch(query).unwrap();
        SqliteFrameDatabase {
            connection,
        }
    }
}

impl FrameDatabase for SqliteFrameDatabase {
    fn add_frame(&self, frame: DbFrame) {
        // Implementation to add a frame to the database
        let query = "INSERT INTO frames (timestamp, speed, rssi, channel, home_id, src_node_id, dst_node_id, payload_raw) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

        match self.connection.execute(
            query,
            rusqlite::params![
                frame.timestamp,
                frame.speed,
                frame.rssi,
                frame.channel,
                frame.home_id,
                frame.src_node_id,
                frame.dst_node_id,
                frame.payload,
            ],
        ) {
            Ok(_) => {
                //println!("Frame added successfully");
            },
            Err(e) => println!("Failed to add frame: {}", e),
        }
    }

    fn get_frame(&self, id: u64) -> Option<DbFrame> {
        let query = "SELECT id, timestamp, speed, rssi, channel, home_id, src_node_id, dst_node_id, payload_raw FROM frames WHERE id = (?1)";

        let mut statement = match self.connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(_e) => {
                //println!("Failed to prepare statement: {}", e);
                return None
            },
        };

        let frame_iter = match statement.query_map([id as i64], |row| {
            Ok(DbFrame {
                id: row.get(0)?,        // ID is selected in this query
                timestamp: row.get(1)?, // Timestamp is selected in this query
                speed: row.get(2)?,     // Speed is selected in this query
                rssi: row.get(3)?,      // RSSI is selected in this query
                channel: row.get(4)?,   // Channel is selected in this query
                home_id: row.get(5)?,   // Home ID is selected in this query
                src_node_id: row.get(6)?, // Source Node ID is selected in this query
                dst_node_id: row.get(7)?, // Destination Node ID is selected in this query
                payload: vec![],   // Payload is selected in this query
            })
        }) {
            Ok(iter) => iter,
            Err(_e) => {
                //println!("Failed to query frame: {}", e);
                return None
            },
        };

        for frame in frame_iter {
            match frame {
                Ok(frame) => {
                    //println!("Found frame with ID {}: {:?}", id, frame);
                    return Some(frame);
                },
                Err(_e) => {
                    //println!("Failed to parse frame: {}", e);
                    return None
                },
            }
        }
        None
    }

    fn get_frames(&self, offset: usize, limit: usize) -> Vec<DbFrame> {
        let query = "SELECT id, timestamp, speed, rssi, channel, home_id, src_node_id, dst_node_id FROM frames LIMIT (?1) OFFSET (?2)";

        let mut statement = match self.connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(_e) => {
                //println!("Failed to prepare statement: {}", e);
                return vec![]
            },
        };

        let frame_iter = match statement.query_map((limit as i64, offset as i64), |row| {
            Ok(DbFrame {
                id: row.get(0)?,        // ID is selected in this query
                timestamp: row.get(1)?, // Timestamp is selected in this query
                speed: row.get(2)?,     // Speed is selected in this query
                rssi: row.get(3)?,      // RSSI is selected in this query
                channel: row.get(4)?,   // Channel is selected in this query
                home_id: row.get(5)?,   // Home ID is selected in this query
                src_node_id: row.get(6)?, // Source Node ID is selected in this query
                dst_node_id: row.get(7)?, // Destination Node ID is selected in this query
                payload: vec![], // Payload is not selected in this query
            })
        }) {
            Ok(iter) => iter,
            Err(_e) => {
                //println!("Failed to query frames: {}", e);
                return vec![]
            },
        };

        let mut frames = Vec::new();

        for frame in frame_iter {
            match frame {
                Ok(frame) => {
                    //println!("Pushing frame: {:?}", frame);
                    frames.push(frame);
                },
                Err(_e) => {
                    //println!("Failed to parse frame: {}", e);
                    return vec![]
                },
            }
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query() {
        let db = SqliteFrameDatabase::new();
        let frame = DbFrame {
            id: 0, // ID will be auto-generated by the database
            timestamp: 1627849800,
            speed: 5,
            rssi: -100,
            channel: 60,
            home_id: 12345,
            src_node_id: 1,
            dst_node_id: 2,
            payload: vec![1, 2, 3, 4],
        };
        db.add_frame(frame);
        /*
        match db.query("SELECT id FROM frames") {
            Ok(result) => println!("Query successful: {}", result),
            Err(e) => println!("Query failed: {}", e),
        };
         */

    }
}

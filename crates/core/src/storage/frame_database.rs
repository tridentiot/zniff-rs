// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
#[derive(Debug)]
pub struct DbFrame {
  pub id: i64,
  pub channel: u8,
  pub speed: u8,
  pub timestamp: i64,
  pub rssi: i8,
  pub home_id: u32,
  pub src_node_id: u8,
  pub dst_node_id: u8,
  pub payload: Vec<u8>,
}

pub trait FrameDatabase {
    fn add_frame(&self, frame: DbFrame);
    fn get_frame(&self, id: u64) -> Option<DbFrame>;
    fn get_frames(&self, offset: usize, limit: usize) -> Vec<DbFrame>;
}

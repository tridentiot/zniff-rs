#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
  pub region: u8,
  pub channel: u8,
  pub speed: u8,
  pub timestamp: u16,
  pub rssi: u8,
  pub payload: Vec<u8>,
}

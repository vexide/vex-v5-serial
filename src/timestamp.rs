/// The epoch of the serial protocols timestamps
pub const J2000_EPOCH: u32 = 946684800;

pub(crate) fn j2000_timestamp() -> u32 {
    (chrono::Utc::now().timestamp() - J2000_EPOCH as i64) as u32
}
use std::time::SystemTime;

/// The epoch of the serial protocols timestamps
pub const J2000_EPOCH: u32 = 946684800;

pub fn j2000_timestamp() -> i32 {
    (SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        - J2000_EPOCH as u128) as i32
}

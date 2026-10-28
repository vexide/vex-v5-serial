pub type AsyncSerialPort = blocking::Unblock<serialport::COMPort>;

impl super::AsyncSerialPortBuilder for serialport::SerialPortBuilder {
    fn open_async(self) -> serialport::Result<AsyncSerialPort> {
        Ok(blocking::Unblock::new(self.open_native()?))
    }
}

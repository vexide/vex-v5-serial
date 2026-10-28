cfg_select! {
    unix => {
        mod unix;
        pub use unix::AsyncSerialPort;
    }
    windows => {
        mod windows;
        pub use windows::AsyncSerialPort;
    }
    _ => {
        std::compile_error!("the 'serial' feature does not support this target.");
    }
}

pub trait AsyncSerialPortBuilder {
    fn open_async(self) -> serialport::Result<AsyncSerialPort>;
}

// Significant portions of this module are derived from the [`async-serial`] crate.
//
// [`async-serial`]: <https://github.com/ZXY595/async-serial>
//
// Copyright (c) 2026 ZXY
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{
    io,
    ops::{Deref, DerefMut},
    os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd},
};

use async_io::{Async, IoSafe};

pub type AsyncSerialPort = Async<IoSafeAdapter<serialport::TTYPort>>;

impl super::AsyncSerialPortBuilder for serialport::SerialPortBuilder {
    fn open_async(self) -> serialport::Result<AsyncSerialPort> {
        let io = IoSafeAdapter(self.open_native()?);
        Ok(Async::new(io)?)
    }
}

/// An [`IoSafe`] newtype for [`serialport::SerialPort`] implementors, useful for [`Async`] wrapper.
#[derive(Debug)]
pub struct IoSafeAdapter<T>(T);

impl<T> Deref for IoSafeAdapter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for IoSafeAdapter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: io::Read> io::Read for IoSafeAdapter<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<T: io::Write> io::Write for IoSafeAdapter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// # Safety:
/// The implementation of [`io::Read`] and [`io::Write`] for the wrapped serial port is [`IoSafe`]
unsafe impl<T: io::Read + io::Write> IoSafe for IoSafeAdapter<T> {}

impl<T: AsRawFd> AsRawFd for IoSafeAdapter<T> {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl<T> AsFd for IoSafeAdapter<T>
where
    Self: AsRawFd,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        // Safety: This fd lifespan is tied to the `T`
        unsafe { BorrowedFd::borrow_raw(self.as_raw_fd()) }
    }
}

// SPDX-FileCopyrightText: 2025 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use bytes::Buf;
use memmap2::{MmapMut, MmapOptions};
use parking_lot::Mutex;

use std::{cmp::Ordering, fs::OpenOptions, io, ops::DerefMut, path::Path};

/// Append-only fixed-size ring buffer backed by a file
///
/// The end of the buffer is marked by a null byte.
#[derive(Debug)]
pub struct FileRingBuffer {
    mmap: MmapMut,
    tail: usize,
}

impl FileRingBuffer {
    pub fn open(file_path: impl AsRef<Path>, capacity: usize) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(file_path)?;

        file.set_len(capacity as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        let mut bytes = mmap.iter();
        let tail = bytes.position(|&b| b == 0).unwrap_or(0);

        Ok(Self { mmap, tail })
    }

    pub fn clear(&mut self) {
        self.mmap.fill(0);
        self.tail = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.tail == 0 && self.mmap[self.tail + 1] == 0
    }

    pub fn data_after_tail(&self) -> bool {
        self.mmap.get(self.tail + 1) != Some(&0)
    }

    fn capacity(&self) -> usize {
        self.mmap.len()
    }

    pub fn len(&self) -> usize {
        if self.mmap.get(self.tail + 1) != Some(&0) {
            self.capacity() - 1
        } else {
            self.tail
        }
    }

    fn write_data(&mut self, data: &[u8]) -> io::Result<()> {
        let capacity = self.capacity();

        if data.len() + 1 > capacity {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Data too long to fit in buffer: data len {} vs capacity {}",
                    data.len(),
                    capacity,
                ),
            ));
        }

        if self.tail + data.len() < capacity {
            // fits
            self.mmap[self.tail..self.tail + data.len()].copy_from_slice(data);
            self.tail += data.len();
        } else {
            // does not fit
            let data_offset = capacity - self.tail;
            self.mmap[self.tail..capacity].copy_from_slice(&data[..data_offset]);
            self.tail = data.len() - data_offset;
            self.mmap[..self.tail].copy_from_slice(&data[data_offset..]);
        }

        self.mmap[self.tail] = 0; // mark end of buffer

        Ok(())
    }

    pub fn buf(&self) -> impl Buf + '_ {
        let pos = if self.data_after_tail() {
            (self.tail + 1) % self.capacity()
        } else {
            0
        };
        RingBufferReader {
            buf: self,
            pos,
            flipped: false,
        }
    }
}

impl io::Write for FileRingBuffer {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.write_data(data)?;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.mmap.flush()
    }
}

#[derive(Debug)]
struct RingBufferReader<'a> {
    buf: &'a FileRingBuffer,
    pos: usize,
    flipped: bool,
}

impl Buf for RingBufferReader<'_> {
    fn remaining(&self) -> usize {
        match self.pos.cmp(&self.buf.tail) {
            Ordering::Less => self.buf.tail - self.pos,
            Ordering::Equal if !self.flipped && self.buf.data_after_tail() => {
                self.buf.capacity() - self.pos - 1
            }
            Ordering::Equal => 0,
            Ordering::Greater => self.buf.capacity() - self.pos,
        }
    }

    fn chunk(&self) -> &[u8] {
        match self.pos.cmp(&self.buf.tail) {
            Ordering::Less => &self.buf.mmap[self.pos..self.buf.tail],
            Ordering::Equal if !self.flipped && self.buf.data_after_tail() => {
                &self.buf.mmap[self.pos + 1..self.buf.capacity()]
            }
            Ordering::Equal => &[],
            Ordering::Greater => &self.buf.mmap[self.pos..self.buf.capacity()],
        }
    }

    fn advance(&mut self, cnt: usize) {
        if self.pos < self.buf.tail || self.pos + cnt < self.buf.capacity() {
            self.pos += cnt;
        } else {
            self.pos = self.pos + cnt - self.buf.capacity();
            self.flipped = true;
        }
    }
}

/// A thread-safe wrapper around [`FileRingBuffer`].
///
/// `Arc<FileRingBufferLock>` can be used as a writer for [`tracing_subscriber::fmt::Subscriber`].
#[derive(Debug)]
pub struct FileRingBufferLock {
    inner: Mutex<FileRingBuffer>,
}

impl FileRingBufferLock {
    pub fn new(buffer: FileRingBuffer) -> Self {
        Self {
            inner: Mutex::new(buffer),
        }
    }

    pub fn lock(&self) -> impl DerefMut<Target = FileRingBuffer> + '_ {
        self.inner.lock()
    }

    pub fn into_inner(self) -> FileRingBuffer {
        self.inner.into_inner()
    }
}

impl io::Write for &FileRingBufferLock {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::{BufRead, Write};

    #[test]
    fn test() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("ring_buffer.dat");

        let mut ring_buffer = FileRingBuffer::open(path, 40)?;
        ring_buffer.clear();

        writeln!(ring_buffer, "Hello, world!").unwrap();
        writeln!(ring_buffer, "This is a test.").unwrap();
        writeln!(ring_buffer, "Another line.").unwrap();

        let mut lines = ring_buffer.buf().reader().lines().map_while(Result::ok);
        assert_eq!(lines.next().unwrap(), ", world!");
        assert_eq!(lines.next().unwrap(), "This is a test.");
        assert_eq!(lines.next().unwrap(), "Another line.");
        assert_eq!(lines.next(), None);

        Ok(())
    }
}

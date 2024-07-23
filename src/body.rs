use bytes::{Bytes, BytesMut};
use pingora::prelude::*;
use std::fs::File;
use std::io::{Read, Seek};

pub const ERR_FILE_READ: ErrorType = ErrorType::new("ERR_FILE_READ");

pub(crate) struct BodyReader {
    fd: File,
    pub(crate) start: u64,
    pub(crate) end: u64,
    bytes_read: usize,
}

impl BodyReader {
    pub fn new(fd: File, total_length: u64) -> Self {
        BodyReader {
            fd,
            start: 0,
            end: total_length as u64,
            bytes_read: 0,
        }
    }

    /// Try to read more data, at most `chunk_size` bytes
    ///
    /// Return `None` when (after) read to the end.
    pub fn read(&mut self, chunk_size: usize) -> Result<Option<Bytes>> {
        // All bytes read
        if self.bytes_read as u64 >= self.end - self.start {
            return Ok(None);
        }

        // nothing read yet, seek() to start at the correct position
        if self.bytes_read == 0 && self.start != 0 {
            self.fd
                .seek(std::io::SeekFrom::Start(self.start))
                .or_err(ERR_FILE_READ, "while seeking()")?;
        }

        // can probably just use Vec<u8>
        let mut data = BytesMut::with_capacity(chunk_size);
        data.resize(chunk_size, 0);

        let read = self
            .fd
            .read(&mut data[0..chunk_size])
            .or_err(ERR_FILE_READ, "while reading the file")?;
        data.resize(read, 0);

        self.bytes_read += read;

        Ok(Some(data.freeze()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let path = std::path::PathBuf::from(format!(
            "{}/tests/files/test.html",
            env!("CARGO_MANIFEST_DIR")
        ));
        let fd = File::open(&path).unwrap();
        let meta = path.metadata().unwrap();

        let mut read = BodyReader::new(fd, meta.len());

        let read0 = read.read(5).unwrap().unwrap();
        assert_eq!(read0, "<!DOC");

        let read1 = read.read(4).unwrap().unwrap();
        assert_eq!(read1, "TYPE");

        let read_rest = read.read(8192).unwrap().unwrap();
        assert!(read_rest.ends_with(b"</html>"));

        let end = read.read(8192).unwrap();
        assert!(end.is_none());
    }

    #[test]
    fn read_seek() {
        let path = std::path::PathBuf::from(format!(
            "{}/tests/files/test.html",
            env!("CARGO_MANIFEST_DIR")
        ));
        let fd = File::open(&path).unwrap();
        let meta = path.metadata().unwrap();

        let mut read = BodyReader::new(fd, meta.len());

        read.start = 5;

        let read1 = read.read(4).unwrap().unwrap();
        assert_eq!(read1, "TYPE");

        let read_rest = read.read(8192).unwrap().unwrap();
        assert!(read_rest.ends_with(b"</html>"));

        let end = read.read(8192).unwrap();
        assert!(end.is_none());
    }
}

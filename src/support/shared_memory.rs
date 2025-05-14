#![allow(dead_code)]
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use memmap2::MmapMut;
use std::mem::{size_of, align_of};
use std::os::unix::fs::OpenOptionsExt;
use thiserror::Error;
use std::time::{Duration, Instant};
use std::ptr::{self, NonNull};

const SHARED_MEM_MAGIC: u32 = 0xDEADBEEF;
const DEFAULT_ALIGNMENT: usize = 64;

const VERBOSE: bool = false;

#[repr(C, align(64))]
struct SharedHeader {
    magic: u32,
    ready: AtomicBool,
    read_pos: AtomicUsize,
    write_pos: AtomicUsize,
    capacity: AtomicUsize,
}

#[derive(Error, Debug)]
pub enum SharedMemoryError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Data too large for shared memory (max: {0}, got: {1})")]
    DataTooLarge(usize, usize),
    #[error("No data available")]
    NoDataAvailable,
    #[error("Timeout while waiting for data")]
    Timeout,
    #[error("Shared memory corrupted")]
    Corrupted,
    #[error("Buffer overflow - reader is too slow")]
    BufferOverflow,
    #[error("Alignment error")]
    AlignmentError,
}

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub struct SharedMemory {
    mmap: MmapMut,
    file: File,
    path: PathBuf,
    header: NonNull<SharedHeader>,
    data_ptr: NonNull<u8>,
    is_creator: bool,
}

impl SharedMemory {
    pub fn create(path: impl AsRef<Path>, initial_size: usize) -> Result<Self, SharedMemoryError> {
        if initial_size == 0 || initial_size % DEFAULT_ALIGNMENT != 0 {
            return Err(SharedMemoryError::AlignmentError);
        }
        
        //create parent
        if path.as_ref().parent().is_some() {
            std::fs::create_dir_all(path.as_ref().parent().unwrap())?;
        }

        let total_size = align_up(size_of::<SharedHeader>() + initial_size, 4096);
        if total_size > isize::MAX as usize {
            return Err(SharedMemoryError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Requested size too large",
            )));
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o660)
            .open(path.as_ref())?;

        file.set_len(total_size as u64)?;

        let mut mmap = unsafe { MmapMut::map_mut(&file)? };
        let header_ptr = mmap.as_mut_ptr() as *mut SharedHeader;

        if (header_ptr as usize) % align_of::<SharedHeader>() != 0 {
            return Err(SharedMemoryError::AlignmentError);
        }

        let data_ptr = unsafe { header_ptr.add(1) as *mut u8 };
        if (data_ptr as usize) % DEFAULT_ALIGNMENT != 0 {
            return Err(SharedMemoryError::AlignmentError);
        }

        unsafe {
            ptr::write(header_ptr, SharedHeader {
                magic: SHARED_MEM_MAGIC,
                ready: AtomicBool::new(false),
                read_pos: AtomicUsize::new(0),
                write_pos: AtomicUsize::new(0),
                capacity: AtomicUsize::new(initial_size),
            });
        }

        mmap.flush()?;

        Ok(Self {
            mmap,
            file,
            path: path.as_ref().to_path_buf(),
            header: NonNull::new(header_ptr).unwrap(),
            data_ptr: NonNull::new(data_ptr).unwrap(),
            is_creator: true,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, SharedMemoryError> {
        let path = path.as_ref();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        let mut mmap = unsafe { MmapMut::map_mut(&file)? };
        let header_ptr = mmap.as_mut_ptr() as *mut SharedHeader;

        unsafe {
            if (*header_ptr).magic != SHARED_MEM_MAGIC {
                return Err(SharedMemoryError::Corrupted);
            }
            let cap = (*header_ptr).capacity.load(Ordering::SeqCst);
            if cap == 0 || cap % DEFAULT_ALIGNMENT != 0 {
                return Err(SharedMemoryError::Corrupted);
            }
        }

        let data_ptr = unsafe { header_ptr.add(1) as *mut u8 };
        if (data_ptr as usize) % DEFAULT_ALIGNMENT != 0 {
            return Err(SharedMemoryError::Corrupted);
        }

        Ok(Self {
            mmap,
            file,
            path: path.to_path_buf(),
            header: NonNull::new(header_ptr).unwrap(),
            data_ptr: NonNull::new(data_ptr).unwrap(),
            is_creator: false,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<(), SharedMemoryError> {
        let header = unsafe { self.header.as_ref() };
        let capacity = header.capacity.load(Ordering::SeqCst);

        if data.len() > capacity {
            return Err(SharedMemoryError::DataTooLarge(capacity, data.len()));
        }

        unsafe {
            let write_pos = header.write_pos.load(Ordering::SeqCst);
            let read_pos = header.read_pos.load(Ordering::SeqCst);

            let available_space = if write_pos >= read_pos {
                capacity - (write_pos - read_pos)
            } else {
                read_pos - write_pos
            };

            if data.len() > available_space {
                return Err(SharedMemoryError::BufferOverflow);
            }

            let buf_start = self.data_ptr.as_ptr();
            let actual_write_pos = write_pos % capacity;
            let remaining_space = capacity - actual_write_pos;

            if data.len() <= remaining_space {
                if VERBOSE {
                    println!("writing {} bytes at {}", data.len(), actual_write_pos);
                }

                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    buf_start.add(actual_write_pos),
                    data.len()
                );
            } else {
                if VERBOSE {
                    println!("!writing {} bytes at {}", remaining_space, actual_write_pos);
                }
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    buf_start.add(actual_write_pos),
                    remaining_space
                );
                if VERBOSE {
                    println!("!writing {} bytes at {}", data.len() - remaining_space, 0);
                }
                ptr::copy_nonoverlapping(
                    data.as_ptr().add(remaining_space),
                    buf_start,
                    data.len() - remaining_space
                );
            }

            header.write_pos.store(write_pos + data.len(), Ordering::Release);
            header.ready.store(true, Ordering::SeqCst);

            if VERBOSE {
                println!("@writing pos  {}", write_pos + data.len());
            }

            self.mmap.flush()?;
        }

        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, SharedMemoryError> {
        self.read_timeout(buf, None)
    }

    pub fn read_timeout(
        &self,
        buf: &mut [u8],
        timeout: Option<Duration>,
    ) -> Result<usize, SharedMemoryError> {
        let start = Instant::now();
        let header = unsafe { self.header.as_ref() };
        let mut sleep_duration = Duration::from_micros(100);

        loop {
            let write_pos = header.write_pos.load(Ordering::Acquire);
            let read_pos = header.read_pos.load(Ordering::Acquire);

            if write_pos > read_pos {
                let available = write_pos - read_pos;
                let to_read = available.min(buf.len());
                let capacity = header.capacity.load(Ordering::Acquire);
                let actual_read_pos = read_pos % capacity;
                let remaining_data = capacity - actual_read_pos;

                unsafe {
                    let buf_start = self.data_ptr.as_ptr();

                    if to_read <= remaining_data {
                        if VERBOSE {
                            println!("reading {} bytes at {}", to_read, actual_read_pos);
                        }
                        ptr::copy_nonoverlapping(
                            buf_start.add(actual_read_pos),
                            buf.as_mut_ptr(),
                            to_read
                        );
                    } else {
                        if VERBOSE {
                            println!("!reading {} bytes at {}", remaining_data, actual_read_pos);
                        }
                        ptr::copy_nonoverlapping(
                            buf_start.add(actual_read_pos),
                            buf.as_mut_ptr(),
                            remaining_data
                        );
                        if VERBOSE {
                            println!("!reading {} bytes at {}", to_read - remaining_data, 0);
                        }
                        ptr::copy_nonoverlapping(
                            buf_start,
                            buf.as_mut_ptr().add(remaining_data),
                            to_read - remaining_data
                        );
                    }
                }

                header.read_pos.store(read_pos + to_read, Ordering::Release);
                if VERBOSE {
                    println!("@reading pos  {}", read_pos + to_read);
                }

                if write_pos == read_pos + to_read {
                    header.ready.store(false, Ordering::SeqCst);
                }

                self.mmap.flush()?;
                return Ok(to_read);
            }

            if let Some(timeout) = timeout {
                if start.elapsed() >= timeout {
                    return Err(SharedMemoryError::Timeout);
                }
                sleep_duration = sleep_duration.min(timeout - start.elapsed());
            }

            std::thread::sleep(sleep_duration);
            sleep_duration = sleep_duration.saturating_mul(2).min(Duration::from_millis(10));
        }
    }

    pub fn try_read(&mut self, buf: &mut [u8]) -> Result<usize, SharedMemoryError> {
        let header = unsafe { self.header.as_ref() };
        let write_pos = header.write_pos.load(Ordering::Acquire);
        let read_pos = header.read_pos.load(Ordering::Acquire);

        if write_pos <= read_pos {
            return Err(SharedMemoryError::NoDataAvailable);
        }

        self.read(buf)
    }

    pub fn capacity(&self) -> usize {
        unsafe { self.header.as_ref().capacity.load(Ordering::Acquire) }
    }

    pub fn available(&self) -> usize {
        let header = unsafe { self.header.as_ref() };
        let write_pos = header.write_pos.load(Ordering::Acquire);
        let read_pos = header.read_pos.load(Ordering::Acquire);
        write_pos - read_pos
    }

    pub fn check_health(&self) -> Result<(), SharedMemoryError> {
        let header = unsafe { self.header.as_ref() };
        if header.magic != SHARED_MEM_MAGIC {
            return Err(SharedMemoryError::Corrupted);
        }
        let cap = header.capacity.load(Ordering::SeqCst);
        if cap == 0 || cap % DEFAULT_ALIGNMENT != 0 {
            return Err(SharedMemoryError::Corrupted);
        }
        Ok(())
    }

    pub fn recover(&mut self) -> Result<(), SharedMemoryError> {
        self.check_health()?;
        let header = unsafe { self.header.as_ref() };
        header.ready.store(false, Ordering::SeqCst);
        Ok(())
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        if self.is_creator {
            if let Err(e) = std::fs::remove_file(&self.path) {
                if cfg!(debug_assertions) {
                    eprintln!("Failed to remove shared memory file: {}", e);
                }
            }
        }
    }
}

unsafe impl Send for SharedMemory {}


pub struct MemoryDuplex {
    reader: SharedMemory,
    writer: SharedMemory,
}

impl MemoryDuplex {
    pub fn create(path: impl AsRef<Path>, initial_size: usize) -> Result<Self, SharedMemoryError> {
        let mut reader_file = path.as_ref().to_string_lossy().to_string();
        reader_file.push_str("_reader");

        let mut writer_file = path.as_ref().to_string_lossy().to_string();
        writer_file.push_str("_writer");

        let reader = SharedMemory::create(reader_file, initial_size)?;
        let writer = SharedMemory::create(writer_file, initial_size)?;
        Ok(Self { reader, writer })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, SharedMemoryError> {
        let mut write_file = path.as_ref().to_string_lossy().to_string();
        write_file.push_str("_reader");

        let mut reader_file = path.as_ref().to_string_lossy().to_string();
        reader_file.push_str("_writer");

        let reader = SharedMemory::open(reader_file)?;
        let writer = SharedMemory::open(write_file)?;
        Ok(Self { reader, writer })
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, SharedMemoryError> {
        self.reader.read(buf)
    }

     pub fn write(&self, data: &[u8]) -> Result<(), SharedMemoryError> {
        self.writer.write(data)
    }
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_shared_memory() {
        let mem = SharedMemory::create("test1", 1024).unwrap();
        let mut buf = vec![0u8; 11];
        mem.write("hello world".as_bytes()).unwrap();

        let rmem = SharedMemory::open("test1").unwrap();
        rmem.read(&mut buf).unwrap();

        assert_eq!(buf, "hello world".as_bytes());
    }

    #[test]
    fn test_shared_memory_wrap() {
        let mem = SharedMemory::create("test2", 128).unwrap();
        let mut buf = vec![0u8; 192];
        for i in 0..192 {
            buf[i] = i as u8;
        }

        mem.write(&buf[0..48]).unwrap();

        let mut rbuf = vec![0u8; 48];
        let rmem = SharedMemory::open("test2").unwrap();
        rmem.read(&mut rbuf).unwrap();

        assert_eq!(&buf[0..48], rbuf);

        mem.write(&buf[48..96]).unwrap();
        rmem.read(&mut rbuf).unwrap();
        assert_eq!(&buf[48..96], rbuf);


        mem.write(&buf[96..144]).unwrap();
        rmem.read(&mut rbuf).unwrap();
        assert_eq!(&buf[96..144], rbuf);

        mem.write(&buf[144..192]).unwrap();
        rmem.read(&mut rbuf).unwrap();
        assert_eq!(&buf[144..192], rbuf);
    }
}
use anyhow::Result;
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_ring_buffer() {
        let path = "/tmp/test_ringbuf_create";
        let _ = fs::remove_file(path); // cleanup if exists

        let writer = RingBufferWriter::new(path, 48000, 2, 1).unwrap();

        // Verify file exists
        assert!(Path::new(path).exists());

        // Verify header values
        assert_eq!(writer.sample_rate, 48000);
        assert_eq!(writer.channels, 2);
        assert_eq!(writer.capacity, 48000); // 1 second

        // Cleanup
        drop(writer);
        fs::remove_file(path).unwrap();
    }
}

pub struct RingBufferWriter {
    sample_rate: u64,
    channels: usize,
    capacity: usize,
}

impl RingBufferWriter {
    pub fn new(
        path: impl AsRef<Path>,
        sample_rate: u64,
        channels: usize,
        duration_secs: u64,
    ) -> Result<Self> {
        let capacity = (sample_rate * duration_secs) as usize;
        let header_size = 4096;
        let data_size = channels * capacity * 8; // 8 bytes per f64
        let total_size = header_size + data_size;

        // Create memory-mapped file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.set_len(total_size as u64)?;

        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        // Write header
        mmap[0..8].copy_from_slice(b"AUDITAB!");
        mmap[8..16].copy_from_slice(&1u64.to_le_bytes()); // version
        mmap[16..24].copy_from_slice(&sample_rate.to_le_bytes());
        mmap[24..32].copy_from_slice(&(channels as u64).to_le_bytes());
        mmap[32..40].copy_from_slice(&(capacity as u64).to_le_bytes());

        // Initialize write_sequence to 0
        let write_seq_ptr = &mut mmap[40..48];
        write_seq_ptr.copy_from_slice(&0u64.to_le_bytes());

        Ok(Self {
            sample_rate,
            channels,
            capacity,
        })
    }
}

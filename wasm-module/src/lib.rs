use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RingBufferReader {
    memory: Vec<u8>,
    sample_rate: u64,
    channels: usize,
    capacity: usize,
}

#[wasm_bindgen]
impl RingBufferReader {
    #[wasm_bindgen(constructor)]
    pub fn new(buffer: &[u8]) -> Self {
        // Buffer length validation
        assert!(buffer.len() >= 4096, "Buffer too small: expected at least 4096 bytes for header");

        // Magic number check
        let magic = &buffer[0..8];
        assert_eq!(magic, b"AUDITAB!", "Invalid magic number: expected 'AUDITAB!'");

        // Parse header
        let sample_rate = u64::from_le_bytes(buffer[16..24].try_into().unwrap());
        let channels = u64::from_le_bytes(buffer[24..32].try_into().unwrap()) as usize;
        let capacity = u64::from_le_bytes(buffer[32..40].try_into().unwrap()) as usize;

        Self {
            memory: buffer.to_vec(),
            sample_rate,
            channels,
            capacity,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> u64 {
        self.sample_rate
    }

    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> usize {
        self.channels
    }

    #[wasm_bindgen]
    pub fn get_waveform(&self, channel: usize, num_points: usize) -> Vec<f64> {
        let ch_offset = 4096 + (channel * self.capacity * 8);
        let decimation = self.capacity / num_points;

        let mut result = Vec::with_capacity(num_points);
        for i in 0..num_points {
            let idx = (i * decimation) % self.capacity;
            let offset = ch_offset + (idx * 8);

            if offset + 8 <= self.memory.len() {
                let sample = f64::from_le_bytes(
                    self.memory[offset..offset + 8].try_into().unwrap()
                );
                result.push(sample);
            }
        }

        result
    }
}

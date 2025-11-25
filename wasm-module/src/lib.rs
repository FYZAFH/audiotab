use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RingBufferReader {
    sample_rate: u64,
    channels: usize,
    capacity: usize,
}

#[wasm_bindgen]
impl RingBufferReader {
    #[wasm_bindgen(constructor)]
    pub fn new(buffer: &[u8]) -> Self {
        // Parse header
        let sample_rate = u64::from_le_bytes(buffer[16..24].try_into().unwrap());
        let channels = u64::from_le_bytes(buffer[24..32].try_into().unwrap()) as usize;
        let capacity = u64::from_le_bytes(buffer[32..40].try_into().unwrap()) as usize;

        Self {
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
}

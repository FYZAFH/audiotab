use std::sync::{Arc, Mutex};

pub struct BufferPool {
    buffers: Arc<Mutex<Vec<Vec<f64>>>>,
    capacity: usize,
}

impl BufferPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(Vec::new())),
            capacity,
        }
    }

    pub fn get(&self) -> PooledBuffer {
        let mut buffers = self.buffers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let buffer = buffers.pop().unwrap_or_else(|| {
            Vec::with_capacity(self.capacity)
        });

        PooledBuffer {
            buffer: Some(buffer),
            pool: self.buffers.clone(),
        }
    }

    pub fn pool_size(&self) -> usize {
        self.buffers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self {
            buffers: self.buffers.clone(),
            capacity: self.capacity,
        }
    }
}

pub struct PooledBuffer {
    buffer: Option<Vec<f64>>,
    pool: Arc<Mutex<Vec<Vec<f64>>>>,
}

impl PooledBuffer {
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map(|b| b.capacity()).unwrap_or(0)
    }

    pub fn as_slice(&self) -> &[f64] {
        self.buffer.as_ref().map(|b| b.as_slice()).unwrap_or(&[])
    }

    pub fn as_mut_slice(&mut self) -> &mut [f64] {
        self.buffer.as_mut().map(|b| b.as_mut_slice()).unwrap_or(&mut [])
    }

    pub fn push(&mut self, value: f64) {
        if let Some(buffer) = &mut self.buffer {
            buffer.push(value);
        }
    }

    pub fn clear(&mut self) {
        if let Some(buffer) = &mut self.buffer {
            buffer.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(mut buffer) = self.buffer.take() {
            buffer.clear();
            let mut pool = self.pool
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pool.push(buffer);
        }
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = Vec<f64>;

    fn deref(&self) -> &Self::Target {
        self.buffer
            .as_ref()
            .expect("PooledBuffer accessed after drop")
    }
}

impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
            .as_mut()
            .expect("PooledBuffer accessed after drop")
    }
}

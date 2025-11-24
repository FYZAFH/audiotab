use super::DeviceSource;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Registry of available device types
pub struct DeviceRegistry {
    sources: HashMap<String, Box<dyn Fn() -> Box<dyn DeviceSource> + Send + Sync>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn register_source<F>(&mut self, device_type: &str, factory: F)
    where
        F: Fn() -> Box<dyn DeviceSource> + Send + Sync + 'static,
    {
        self.sources.insert(device_type.to_string(), Box::new(factory));
    }

    pub fn create_source(&self, device_type: &str) -> Result<Box<dyn DeviceSource>> {
        self.sources
            .get(device_type)
            .ok_or_else(|| anyhow!("Unknown device type: {}", device_type))
            .map(|factory| factory())
    }

    pub fn list_sources(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

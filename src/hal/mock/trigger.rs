use crate::hal::{DeviceSource, DeviceState};
use crate::core::DataFrame;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Clone)]
enum TriggerMode {
    Periodic { interval_ms: u64 },
    Manual,
}

pub struct SimulatedTriggerSource {
    state: DeviceState,
    mode: TriggerMode,
    frame_counter: u64,
    start_time: Option<Instant>,
    manual_trigger_tx: Option<mpsc::Sender<()>>,
    manual_trigger_rx: Option<mpsc::Receiver<()>>,
}

impl SimulatedTriggerSource {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        Self {
            state: DeviceState::Unopened,
            mode: TriggerMode::Periodic { interval_ms: 1000 },
            frame_counter: 0,
            start_time: None,
            manual_trigger_tx: Some(tx),
            manual_trigger_rx: Some(rx),
        }
    }

    /// Manually trigger a frame (only works in Manual mode)
    pub fn trigger(&self) {
        if let Some(tx) = &self.manual_trigger_tx {
            let _ = tx.try_send(());
        }
    }
}

#[async_trait]
impl DeviceSource for SimulatedTriggerSource {
    async fn configure(&mut self, config: Value) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot configure device in state {:?}", self.state));
        }

        let mode = config["mode"].as_str().unwrap_or("periodic");

        self.mode = match mode {
            "periodic" => {
                let interval_ms = config["interval_ms"].as_u64().unwrap_or(1000);
                TriggerMode::Periodic { interval_ms }
            }
            "manual" => TriggerMode::Manual,
            _ => return Err(anyhow!("Unknown trigger mode: {}", mode)),
        };

        Ok(())
    }

    async fn open(&mut self) -> Result<()> {
        if self.state != DeviceState::Unopened {
            return Err(anyhow!("Cannot open device in state {:?}", self.state));
        }
        self.state = DeviceState::Opened;
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        if self.state != DeviceState::Opened && self.state != DeviceState::Stopped {
            return Err(anyhow!("Cannot start device in state {:?}", self.state));
        }
        self.state = DeviceState::Running;
        self.start_time = Some(Instant::now());
        self.frame_counter = 0;
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<DataFrame> {
        if self.state != DeviceState::Running {
            return Err(anyhow!("Device not running"));
        }

        match self.mode {
            TriggerMode::Periodic { interval_ms } => {
                sleep(Duration::from_millis(interval_ms)).await;
            }
            TriggerMode::Manual => {
                // Wait for manual trigger
                if let Some(rx) = &mut self.manual_trigger_rx {
                    rx.recv().await.ok_or_else(|| anyhow!("Manual trigger channel closed"))?;
                }
            }
        }

        let elapsed = self.start_time.unwrap().elapsed();
        let timestamp = elapsed.as_micros() as u64;

        let mut frame = DataFrame::new(timestamp, self.frame_counter);

        let mode_str = match self.mode {
            TriggerMode::Periodic { .. } => "periodic",
            TriggerMode::Manual => "manual",
        };
        frame.metadata.insert("trigger_mode".to_string(), mode_str.to_string());

        self.frame_counter += 1;

        Ok(frame)
    }

    async fn stop(&mut self) -> Result<()> {
        if self.state != DeviceState::Running {
            return Ok(());
        }
        self.state = DeviceState::Stopped;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if self.state == DeviceState::Running {
            self.stop().await?;
        }
        if self.state != DeviceState::Closed {
            self.state = DeviceState::Closed;
        }
        Ok(())
    }

    fn state(&self) -> DeviceState {
        self.state.clone()
    }
}

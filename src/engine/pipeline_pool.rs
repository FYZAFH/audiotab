use anyhow::Result;
use serde_json::Value;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use std::sync::Arc;
use crate::core::DataFrame;
use super::AsyncPipeline;

pub struct PipelinePool {
    config: Value,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl PipelinePool {
    pub async fn new(config: Value, max_concurrent: usize) -> Result<Self> {
        // Validate config by creating one pipeline
        let _test_pipeline = AsyncPipeline::from_json(config.clone()).await?;

        Ok(Self {
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        })
    }

    pub async fn execute(&mut self, trigger_frame: DataFrame) -> Result<JoinHandle<Result<()>>> {
        let config = self.config.clone();
        let semaphore = self.semaphore.clone();

        let handle = tokio::spawn(async move {
            // Acquire permit (blocks if max_concurrent already running)
            let _permit = semaphore.acquire().await.unwrap();

            // Create and run pipeline instance
            let mut pipeline = AsyncPipeline::from_json(config).await?;
            pipeline.start().await?;
            pipeline.trigger(trigger_frame).await?;

            // Wait a bit for processing to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            pipeline.stop().await?;
            // Permit is dropped here, allowing next pipeline to start

            Ok(())
        });

        Ok(handle)
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

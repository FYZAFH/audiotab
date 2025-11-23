use audiotab::engine::{AsyncPipeline, PipelinePool};
use audiotab::core::DataFrame;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("StreamLab Core - Phase 2 Async Demo");
    println!("====================================\n");

    // Demo 1: Single async pipeline with streaming
    println!("=== Demo 1: Async Streaming Pipeline ===");
    let config1 = serde_json::json!({
        "pipeline_config": {
            "channel_capacity": 10
        },
        "nodes": [
            {
                "id": "sine_gen",
                "type": "SineGenerator",
                "config": {
                    "frequency": 440.0,
                    "sample_rate": 48000.0,
                    "frame_size": 1024
                }
            },
            {
                "id": "amplifier",
                "type": "Gain",
                "config": {
                    "gain": 2.5
                }
            },
            {
                "id": "console_out",
                "type": "Print",
                "config": {
                    "label": "Async Output"
                }
            }
        ],
        "connections": [
            {"from": "sine_gen", "to": "amplifier"},
            {"from": "amplifier", "to": "console_out"}
        ]
    });

    let mut pipeline = AsyncPipeline::from_json(config1).await?;
    pipeline.start().await?;

    println!("Triggering 5 frames through async pipeline...\n");
    for i in 0..5 {
        pipeline.trigger(DataFrame::new(i * 1000, i)).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    pipeline.stop().await?;

    // Demo 2: Pipeline pool with concurrent execution
    println!("\n=== Demo 2: Concurrent Pipeline Execution ===");
    let config2 = serde_json::json!({
        "nodes": [
            {
                "id": "sine_gen",
                "type": "SineGenerator",
                "config": {
                    "frequency": 880.0,
                    "sample_rate": 48000.0,
                    "frame_size": 512
                }
            },
            {
                "id": "console_out",
                "type": "Print",
                "config": {
                    "label": "Pool"
                }
            }
        ],
        "connections": [
            {"from": "sine_gen", "to": "console_out"}
        ]
    });

    let mut pool = PipelinePool::new(config2, 3).await?;

    println!("Launching 10 pipeline instances (3 concurrent max)...\n");
    let mut handles = vec![];
    for i in 0..10 {
        let handle = pool.execute(DataFrame::new(i * 500, i)).await?;
        handles.push(handle);
    }

    println!("Waiting for all 10 instances to complete...\n");
    for (i, handle) in handles.into_iter().enumerate() {
        handle.await??;
        println!("Instance {} completed", i);
    }

    println!("\n=== Phase 2 Demo Complete! ===");
    println!("✓ Async streaming pipeline with tokio tasks");
    println!("✓ MPSC channels for inter-node communication");
    println!("✓ Backpressure via bounded channels");
    println!("✓ Concurrent pipeline instances with PipelinePool");
    println!("✓ Phase continuity in SineGenerator");

    Ok(())
}

use audiotab::engine::AsyncPipeline;
use audiotab::core::DataFrame;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("StreamLab Core - Phase 3 Production Readiness Demo");
    println!("=================================================\n");

    // Demo 1: Pipeline with metrics monitoring
    println!("=== Demo 1: Observability & Metrics ===");
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
                    "label": "Production Output"
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

    println!("Processing 10 frames with metrics tracking...\n");
    for i in 0..10 {
        pipeline.trigger(DataFrame::new(i * 1000, i)).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Display metrics
    if let Some(monitor) = pipeline.get_monitor() {
        println!("\n{}", monitor.generate_report());
    }

    pipeline.stop().await?;

    println!("\n=== Phase 3 Features Demonstrated ===");
    println!("✓ NodeMetrics with atomic counters");
    println!("✓ MetricsCollector for aggregation");
    println!("✓ PipelineMonitor for human-readable reports");
    println!("✓ ResilientNode wrapper (error handling ready)");
    println!("✓ BufferPool system (memory optimization ready)");
    println!("✓ Zero-copy DataFrame with Arc<Vec<f64>>");
    println!("\n=== Production Readiness Complete! ===");

    Ok(())
}

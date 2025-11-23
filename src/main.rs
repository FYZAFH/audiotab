pub mod core;
pub mod engine;
pub mod nodes;

use engine::Pipeline;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("StreamLab Core - Phase 1 Demo");
    println!("================================\n");

    // Define a simple pipeline: SineWave -> Gain -> Print
    let config = serde_json::json!({
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
                    "label": "Final Output"
                }
            }
        ],
        "connections": [
            {"from": "sine_gen", "to": "amplifier"},
            {"from": "amplifier", "to": "console_out"}
        ]
    });

    println!("Building pipeline from config...");
    let mut pipeline = Pipeline::from_json(config).await?;

    println!("Executing pipeline 3 times...\n");
    for i in 0..3 {
        println!("--- Execution {} ---", i + 1);
        pipeline.execute_once().await?;
        println!();
    }

    println!("Demo complete! Phase 1 objectives achieved:");
    println!("✓ DataFrame structure defined");
    println!("✓ ProcessingNode trait implemented");
    println!("✓ Three basic nodes: SineGenerator, Gain, Print");
    println!("✓ Pipeline builder parses JSON configuration");
    println!("✓ Linear pipeline execution works");

    Ok(())
}

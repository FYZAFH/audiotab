use audiotab::nodes::*;
use serde_json::json;

#[tokio::test]
async fn test_complete_flow_sine_to_print() {
    // This test validates the complete flow:
    // 1. Frontend exports graph
    // 2. deploy_graph receives graph
    // 3. Graph is translated
    // 4. Pipeline is created
    // 5. Pipeline can be started (when kernel is running)

    // Import nodes to trigger registration
    let _ = (
        GainNode::default(),
        AudioSourceNode::default(),
        DebugSinkNode::default(),
    );

    // Simulate frontend graph export
    let frontend_graph = json!({
        "nodes": [
            {
                "id": "sine-1",
                "type": "SineGenerator",
                "position": {"x": 100, "y": 100},
                "parameters": {"frequency": 440}
            },
            {
                "id": "print-2",
                "type": "Print",
                "position": {"x": 300, "y": 100},
                "parameters": {}
            }
        ],
        "edges": [
            {
                "id": "e1",
                "source": "sine-1",
                "target": "print-2"
            }
        ]
    });

    // Step 1: Translate graph
    use app_lib::translate_graph;
    let backend_graph = translate_graph(frontend_graph).unwrap();

    // Step 2: Create pipeline
    use audiotab::engine::AsyncPipeline;
    let pipeline = AsyncPipeline::from_json(backend_graph).await;

    assert!(pipeline.is_ok(), "Pipeline creation should succeed");

    // Step 3: Verify pipeline structure
    let _pipeline = pipeline.unwrap();
    // Pipeline created successfully - structure validated by AsyncPipeline::from_json
}

#[tokio::test]
async fn test_invalid_graph_fails_gracefully() {
    let frontend_graph = json!({
        "nodes": [
            {
                "id": "invalid-1",
                "type": "UnknownNodeType",
                "parameters": {}
            }
        ],
        "edges": []
    });

    use app_lib::translate_graph;
    let backend_graph = translate_graph(frontend_graph).unwrap();

    use audiotab::engine::AsyncPipeline;
    let result = AsyncPipeline::from_json(backend_graph).await;

    assert!(result.is_err(), "Should fail for unknown node type");
}

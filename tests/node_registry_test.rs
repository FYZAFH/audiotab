use audiotab::registry::{NodeMetadata, NodeMetadataFactoryWrapper};

#[test]
fn test_inventory_collects_all_nodes() {
    // Force import of all nodes
    use audiotab::nodes::*;
    let _ = (
        GainNode::default(),
        AudioSourceNode::default(),
        TriggerSourceNode::default(),
        DebugSinkNode::default(),
        FFTNode::default(),
        FilterNode::default(),
    );

    // Collect from inventory
    let mut nodes: Vec<NodeMetadata> = Vec::new();
    for wrapper in inventory::iter::<NodeMetadataFactoryWrapper> {
        nodes.push((wrapper.0)());
    }

    assert!(nodes.len() >= 6, "Expected at least 6 nodes, found {}", nodes.len());

    // Check specific nodes exist
    let node_ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    assert!(node_ids.contains(&"gainnode"), "GainNode not found");
    assert!(node_ids.contains(&"audiosourcenode"), "AudioSourceNode not found");
    assert!(node_ids.contains(&"triggersourcenode"), "TriggerSourceNode not found");
    assert!(node_ids.contains(&"debugsinknode"), "DebugSinkNode not found");
    assert!(node_ids.contains(&"fftnode"), "FFTNode not found");
    assert!(node_ids.contains(&"filternode"), "FilterNode not found");
}

#[test]
fn test_node_metadata_has_correct_structure() {
    use audiotab::nodes::GainNode;
    let _ = GainNode::default();

    let mut nodes: Vec<NodeMetadata> = Vec::new();
    for wrapper in inventory::iter::<NodeMetadataFactoryWrapper> {
        nodes.push((wrapper.0)());
    }

    let gain_node = nodes.iter().find(|n| n.id == "gainnode").expect("GainNode not found");

    assert_eq!(gain_node.name, "Gain");
    assert_eq!(gain_node.category, "Processors");
    assert_eq!(gain_node.inputs.len(), 1);
    assert_eq!(gain_node.outputs.len(), 1);
    assert!(gain_node.parameters.len() > 0, "Expected parameters");

    // Check gain_db parameter exists
    let gain_param = gain_node.parameters.iter()
        .find(|p| p.name == "gain_db")
        .expect("gain_db parameter not found");

    assert_eq!(gain_param.param_type, "number");
    assert_eq!(gain_param.min, Some(0.0));
    assert_eq!(gain_param.max, Some(80.0));
}

#[test]
fn test_node_factory_creates_instance() {
    use audiotab::nodes::GainNode;
    let _ = GainNode::default();

    let mut nodes: Vec<NodeMetadata> = Vec::new();
    for wrapper in inventory::iter::<NodeMetadataFactoryWrapper> {
        nodes.push((wrapper.0)());
    }

    let gain_node = nodes.iter().find(|n| n.id == "gainnode").expect("GainNode not found");

    // Verify factory creates an instance without panicking
    let instance = gain_node.create_instance();

    // The factory returns a Box<dyn ProcessingNode>, so we verify
    // that the instance was created successfully by testing it's not null
    // (Rust doesn't have null, so if we got here, creation succeeded)
    let _ = instance; // Just verify instance was created
}

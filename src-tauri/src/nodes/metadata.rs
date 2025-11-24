use crate::state::{NodeMetadata, PortMetadata};
use serde_json::json;

pub fn audio_source_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "audio_source".to_string(),
        name: "Audio Source".to_string(),
        category: "Sources".to_string(),
        inputs: vec![],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Audio Out".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        parameters: json!({
            "sample_rate": { "type": "number", "default": 48000 },
            "buffer_size": { "type": "number", "default": 1024 },
        }),
    }
}

pub fn trigger_source_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "trigger_source".to_string(),
        name: "Trigger Source".to_string(),
        category: "Sources".to_string(),
        inputs: vec![],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Trigger Out".to_string(),
            data_type: "trigger".to_string(),
        }],
        parameters: json!({
            "mode": { "type": "string", "default": "periodic" },
            "interval_ms": { "type": "number", "default": 100 },
        }),
    }
}

pub fn debug_sink_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "debug_sink".to_string(),
        name: "Debug Sink".to_string(),
        category: "Sinks".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Data In".to_string(),
            data_type: "any".to_string(),
        }],
        outputs: vec![],
        parameters: json!({
            "log_level": { "type": "string", "default": "info" },
        }),
    }
}

pub fn fft_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "fft".to_string(),
        name: "FFT".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Audio In".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "FFT Out".to_string(),
            data_type: "fft_result".to_string(),
        }],
        parameters: json!({
            "window_type": { "type": "string", "default": "hann" },
        }),
    }
}

pub fn gain_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "gain".to_string(),
        name: "Gain".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Audio In".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Audio Out".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        parameters: json!({
            "gain_db": { "type": "number", "default": 0.0 },
        }),
    }
}

pub fn filter_node_metadata() -> NodeMetadata {
    NodeMetadata {
        id: "filter".to_string(),
        name: "Filter".to_string(),
        category: "Processors".to_string(),
        inputs: vec![PortMetadata {
            id: "input".to_string(),
            name: "Audio In".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        outputs: vec![PortMetadata {
            id: "output".to_string(),
            name: "Audio Out".to_string(),
            data_type: "audio_frame".to_string(),
        }],
        parameters: json!({
            "type": { "type": "string", "default": "lowpass" },
            "cutoff_hz": { "type": "number", "default": 1000.0 },
        }),
    }
}

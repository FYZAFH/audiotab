use audiotab::hal::registry::DeviceRegistry;
use audiotab::hal::mock::{SimulatedAudioSource, SimulatedTriggerSource};

#[tokio::test]
async fn test_registry_with_mock_devices() {
    let mut registry = DeviceRegistry::new();

    // Register mock devices
    registry.register_source("SimulatedAudio", || Box::new(SimulatedAudioSource::new()));
    registry.register_source("SimulatedTrigger", || Box::new(SimulatedTriggerSource::new()));

    // List available sources
    let sources = registry.list_sources();
    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&"SimulatedAudio".to_string()));

    // Create an audio source
    let audio = registry.create_source("SimulatedAudio").unwrap();
    assert_eq!(audio.state(), audiotab::hal::DeviceState::Unopened);

    // Create a trigger source
    let trigger = registry.create_source("SimulatedTrigger").unwrap();
    assert_eq!(trigger.state(), audiotab::hal::DeviceState::Unopened);

    // Try to create unknown device
    let result = registry.create_source("NonExistent");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_default_devices() {
    let registry = DeviceRegistry::with_defaults();

    // Should have mock devices pre-registered
    let sources = registry.list_sources();
    assert!(sources.contains(&"SimulatedAudio".to_string()));
    assert!(sources.contains(&"SimulatedTrigger".to_string()));
}

use audiotab::resilience::ErrorPolicy;
use audiotab::core::DataFrame;

#[test]
fn test_error_policy_propagate() {
    let policy = ErrorPolicy::Propagate;

    match policy {
        ErrorPolicy::Propagate => { /* expected */ }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_error_policy_skip_frame() {
    let policy = ErrorPolicy::SkipFrame;

    match policy {
        ErrorPolicy::SkipFrame => { /* expected */ }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_error_policy_use_default() {
    let default_frame = DataFrame::new(0, 0);
    let policy = ErrorPolicy::UseDefault(default_frame.clone());

    match policy {
        ErrorPolicy::UseDefault(frame) => {
            assert_eq!(frame.timestamp, default_frame.timestamp);
        }
        _ => panic!("Wrong variant"),
    }
}

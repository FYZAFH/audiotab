use audiotab::hal::{ChannelMapper, ChannelMapping, ChannelRoute};

#[test]
fn test_identity_mapping() {
    let mapping = ChannelMapping {
        physical_channels: 4,
        virtual_channels: 4,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Direct(1),
            ChannelRoute::Direct(2),
            ChannelRoute::Direct(3),
        ],
    };

    let physical = vec![1.0, 2.0, 3.0, 4.0];
    let virtual_samples = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_samples, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_reordering() {
    let mapping = ChannelMapping {
        physical_channels: 4,
        virtual_channels: 4,
        routing: vec![
            ChannelRoute::Reorder(vec![2]),
            ChannelRoute::Reorder(vec![0]),
            ChannelRoute::Reorder(vec![3]),
            ChannelRoute::Reorder(vec![1]),
        ],
    };

    let physical = vec![1.0, 2.0, 3.0, 4.0];
    let virtual_samples = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_samples, vec![3.0, 1.0, 4.0, 2.0]);
}

#[test]
fn test_selection_subset() {
    let mapping = ChannelMapping {
        physical_channels: 4,
        virtual_channels: 2,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Direct(2),
        ],
    };

    let physical = vec![1.0, 2.0, 3.0, 4.0];
    let virtual_samples = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_samples, vec![1.0, 3.0]);
}

#[test]
fn test_merging_average() {
    let mapping = ChannelMapping {
        physical_channels: 4,
        virtual_channels: 1,
        routing: vec![
            ChannelRoute::Merge(vec![0, 1, 2, 3]),
        ],
    };

    let physical = vec![1.0, 3.0, 5.0, 7.0];
    let virtual_samples = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_samples, vec![4.0]); // (1+3+5+7)/4 = 4.0
}

#[test]
fn test_duplication() {
    let mapping = ChannelMapping {
        physical_channels: 2,
        virtual_channels: 4,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Direct(1),
            ChannelRoute::Duplicate(0),
            ChannelRoute::Duplicate(1),
        ],
    };

    let physical = vec![1.0, 2.0];
    let virtual_samples = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_samples, vec![1.0, 2.0, 1.0, 2.0]);
}

#[test]
fn test_complex_mapping() {
    // Physical: [L, R, C, LFE]
    // Virtual: [Mono (L+R avg), C, LFE, LFE-duplicate]
    let mapping = ChannelMapping {
        physical_channels: 4,
        virtual_channels: 4,
        routing: vec![
            ChannelRoute::Merge(vec![0, 1]),  // L+R average
            ChannelRoute::Direct(2),           // C passthrough
            ChannelRoute::Direct(3),           // LFE passthrough
            ChannelRoute::Duplicate(3),        // LFE duplicate
        ],
    };

    let physical = vec![2.0, 4.0, 3.0, 1.0];
    let virtual_samples = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_samples, vec![3.0, 3.0, 1.0, 1.0]);
}

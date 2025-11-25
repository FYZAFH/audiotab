use audiotab::hal::*;

#[test]
fn test_channel_mapping_direct() {
    let mapping = ChannelMapping {
        physical_channels: 3,
        virtual_channels: 3,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Direct(1),
            ChannelRoute::Direct(2),
        ],
    };

    let physical = vec![1.0, 2.0, 3.0];  // Interleaved: [ch0, ch1, ch2]
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped.len(), 3);
    assert_eq!(virtual_mapped, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_channel_mapping_reorder() {
    let mapping = ChannelMapping {
        physical_channels: 3,
        virtual_channels: 3,
        routing: vec![
            ChannelRoute::Direct(2),  // Virt[0] = Phys[2]
            ChannelRoute::Direct(1),  // Virt[1] = Phys[1]
            ChannelRoute::Direct(0),  // Virt[2] = Phys[0]
        ],
    };

    let physical = vec![1.0, 2.0, 3.0];
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped, vec![3.0, 2.0, 1.0]);  // Reversed
}

#[test]
fn test_channel_mapping_merge() {
    let mapping = ChannelMapping {
        physical_channels: 3,
        virtual_channels: 1,
        routing: vec![
            ChannelRoute::Merge(vec![0, 1, 2]),  // Virt[0] = avg(Phys[0,1,2])
        ],
    };

    let physical = vec![1.0, 2.0, 3.0];
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped.len(), 1);
    assert_eq!(virtual_mapped[0], 2.0);  // (1+2+3)/3 = 2.0
}

#[test]
fn test_channel_mapping_duplicate() {
    let mapping = ChannelMapping {
        physical_channels: 1,
        virtual_channels: 3,
        routing: vec![
            ChannelRoute::Direct(0),
            ChannelRoute::Duplicate(0),  // Virt[1] = Phys[0]
            ChannelRoute::Duplicate(0),  // Virt[2] = Phys[0]
        ],
    };

    let physical = vec![5.0];
    let virtual_mapped = ChannelMapper::apply(&mapping, &physical).unwrap();

    assert_eq!(virtual_mapped, vec![5.0, 5.0, 5.0]);
}

use anyhow::Result;
use super::types::{ChannelMapping, ChannelRoute};

pub struct ChannelMapper;

impl ChannelMapper {
    /// Apply channel mapping to physical samples, producing virtual samples
    pub fn apply(mapping: &ChannelMapping, physical: &[f64]) -> Result<Vec<f64>> {
        if physical.len() != mapping.physical_channels {
            anyhow::bail!(
                "Expected {} physical channels, got {}",
                mapping.physical_channels,
                physical.len()
            );
        }

        let mut virtual_samples = Vec::with_capacity(mapping.virtual_channels);

        for route in &mapping.routing {
            let sample = match route {
                ChannelRoute::Direct(ch) => {
                    Self::validate_channel(*ch, physical.len())?;
                    physical[*ch]
                }
                ChannelRoute::Reorder(channels) => {
                    // Same as Direct for single channel
                    if channels.len() != 1 {
                        anyhow::bail!("Reorder expects single channel, got {}", channels.len());
                    }
                    Self::validate_channel(channels[0], physical.len())?;
                    physical[channels[0]]
                }
                ChannelRoute::Merge(channels) => {
                    // Average the channels
                    let sum: f64 = channels.iter()
                        .map(|&ch| {
                            Self::validate_channel(ch, physical.len()).unwrap();
                            physical[ch]
                        })
                        .sum();
                    sum / channels.len() as f64
                }
                ChannelRoute::Duplicate(ch) => {
                    Self::validate_channel(*ch, physical.len())?;
                    physical[*ch]
                }
            };

            virtual_samples.push(sample);
        }

        if virtual_samples.len() != mapping.virtual_channels {
            anyhow::bail!(
                "Mapping produced {} channels, expected {}",
                virtual_samples.len(),
                mapping.virtual_channels
            );
        }

        Ok(virtual_samples)
    }

    fn validate_channel(ch: usize, available: usize) -> Result<()> {
        if ch >= available {
            anyhow::bail!("Channel {} out of range (0..{})", ch, available);
        }
        Ok(())
    }

    /// Create default 1:1 mapping
    pub fn default_mapping(num_channels: usize) -> ChannelMapping {
        ChannelMapping {
            physical_channels: num_channels,
            virtual_channels: num_channels,
            routing: (0..num_channels).map(ChannelRoute::Direct).collect(),
        }
    }
}

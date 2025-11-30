import { invoke } from '@tauri-apps/api/core';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

export interface DeviceInfo {
  id: string;
  name: string;
  hardware_type: 'Acoustic' | 'Special';
  driver_id: string;
}

export interface SampleFormat {
  type: 'I16' | 'I24' | 'I32' | 'F32' | 'F64' | 'U8';
}

export interface ChannelRoute {
  Direct?: number;
  Reorder?: number[];
  Merge?: number[];
  Duplicate?: number;
}

export interface ChannelMapping {
  physical_channels: number;
  virtual_channels: number;
  routing: ChannelRoute[];
}

export interface Calibration {
  gain: number;
  offset: number;
}

export interface DeviceConfig {
  name: string;
  sample_rate: number;
  format: SampleFormat;
  buffer_size: number;
  channel_mapping: ChannelMapping;
  calibration: Calibration;
}

export interface DeviceMetadata {
  description?: string;
  tags: string[];
  created_at: number;
  modified_at: number;
}

export interface DeviceProfile {
  id: string;
  alias: string;
  driver_id: string;
  device_id: string;
  config: DeviceConfig;
  metadata: DeviceMetadata;
}

export function useDiscoverDevices() {
  return useQuery<DeviceInfo[]>({
    queryKey: ['devices', 'discover'],
    queryFn: () => invoke<DeviceInfo[]>('discover_devices'),
    // Don't auto-refetch - discovery is expensive
    refetchOnWindowFocus: false,
    refetchOnMount: false,
  });
}

export function useDeviceProfiles() {
  return useQuery<DeviceProfile[]>({
    queryKey: ['devices', 'profiles'],
    queryFn: () => invoke<DeviceProfile[]>('list_device_profiles'),
  });
}

export function useDeviceProfile(id: string | null) {
  return useQuery<DeviceProfile>({
    queryKey: ['devices', 'profile', id],
    queryFn: () => invoke<DeviceProfile>('get_device_profile', { id }),
    enabled: !!id,
  });
}

export function useAddDeviceProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (profile: DeviceProfile) =>
      invoke<void>('add_device_profile', { profile }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['devices', 'profiles'] });
    },
  });
}

export function useUpdateDeviceProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (profile: DeviceProfile) =>
      invoke<void>('update_device_profile', { profile }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['devices', 'profiles'] });
    },
  });
}

export function useDeleteDeviceProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) =>
      invoke<void>('delete_device_profile', { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['devices', 'profiles'] });
    },
  });
}

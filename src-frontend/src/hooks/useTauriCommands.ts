import { invoke } from '@tauri-apps/api/core';
import { useQuery, useMutation } from '@tanstack/react-query';
import type { NodeMetadata, GraphJson, PipelineStatus, PipelineAction } from '../types/nodes';
import type { KernelStatusResponse } from '../types/kernel';

export function useNodeRegistry() {
  return useQuery({
    queryKey: ['nodes'],
    queryFn: () => invoke<NodeMetadata[]>('get_node_registry'),
  });
}

export function useDeployGraph() {
  return useMutation({
    mutationFn: (graph: GraphJson) => invoke<string>('deploy_graph', { graph }),
  });
}

export function usePipelineStates() {
  return useQuery({
    queryKey: ['pipeline-states'],
    queryFn: () => invoke<PipelineStatus[]>('get_all_pipeline_states'),
  });
}

export function useControlPipeline() {
  return useMutation({
    mutationFn: ({ id, action }: { id: string; action: PipelineAction }) =>
      invoke<void>('control_pipeline', { id, action }),
  });
}

// Kernel management hooks

export function useKernelStatus() {
  return useQuery({
    queryKey: ['kernel-status'],
    queryFn: () => invoke<KernelStatusResponse>('get_kernel_status'),
    refetchInterval: 2000, // Poll every 2 seconds
  });
}

export function useStartKernel() {
  return useMutation({
    mutationFn: () => invoke<KernelStatusResponse>('start_kernel'),
  });
}

export function useStopKernel() {
  return useMutation({
    mutationFn: () => invoke<KernelStatusResponse>('stop_kernel'),
  });
}

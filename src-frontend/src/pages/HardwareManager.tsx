import React from 'react';
import { DeviceList } from '../components/DeviceList';

export function HardwareManager() {
  return (
    <div style={{ width: '100%', height: '100vh', overflow: 'auto' }}>
      <DeviceList />
    </div>
  );
}

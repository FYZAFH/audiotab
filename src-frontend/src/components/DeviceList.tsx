import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface DeviceInfo {
  id: string;
  name: string;
  hardware_type: 'Acoustic' | 'Special';
  driver_id: string;
}

export function DeviceList() {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const discoverDevices = async () => {
    setLoading(true);
    try {
      const discovered = await invoke<DeviceInfo[]>('discover_hardware');
      setDevices(discovered);
    } catch (err) {
      console.error('Failed to discover devices:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    discoverDevices();
  }, []);

  const acousticDevices = devices.filter(d => d.hardware_type === 'Acoustic');
  const specialDevices = devices.filter(d => d.hardware_type === 'Special');

  if (loading) {
    return <div>Discovering devices...</div>;
  }

  return (
    <div style={{ padding: '20px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
        <h2>Hardware Manager</h2>
        <button onClick={discoverDevices}>Refresh</button>
      </div>

      <section>
        <h3>🎤 Acoustic Hardware</h3>
        {acousticDevices.length === 0 ? (
          <p>No acoustic devices found</p>
        ) : (
          <ul>
            {acousticDevices.map(device => (
              <li key={device.id}>
                {device.name}
                <button style={{ marginLeft: '10px' }}>Configure</button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section style={{ marginTop: '30px' }}>
        <h3>⚙️ Special Hardware</h3>
        {specialDevices.length === 0 ? (
          <p>No special devices found</p>
        ) : (
          <ul>
            {specialDevices.map(device => (
              <li key={device.id}>
                {device.name}
                <button style={{ marginLeft: '10px' }}>Configure</button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from './ui/button';

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
    return <div className="text-slate-400 p-5">Discovering devices...</div>;
  }

  return (
    <div className="p-5">
      <div className="flex justify-between items-center mb-5">
        <h2 className="text-white text-2xl font-bold">Hardware Manager</h2>
        <Button onClick={discoverDevices} variant="outline">Refresh</Button>
      </div>

      <section className="mb-8">
        <h3 className="text-white text-xl font-semibold mb-3">🎤 Acoustic Hardware</h3>
        {acousticDevices.length === 0 ? (
          <p className="text-slate-400">No acoustic devices found</p>
        ) : (
          <ul className="space-y-2">
            {acousticDevices.map(device => (
              <li key={device.id} className="flex items-center justify-between bg-slate-800 p-3 rounded">
                <span className="text-white">{device.name}</span>
                <Button variant="outline" size="sm">Configure</Button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h3 className="text-white text-xl font-semibold mb-3">⚙️ Special Hardware</h3>
        {specialDevices.length === 0 ? (
          <p className="text-slate-400">No special devices found</p>
        ) : (
          <ul className="space-y-2">
            {specialDevices.map(device => (
              <li key={device.id} className="flex items-center justify-between bg-slate-800 p-3 rounded">
                <span className="text-white">{device.name}</span>
                <Button variant="outline" size="sm">Configure</Button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

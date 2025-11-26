import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Card } from './ui/card';

// Type definitions matching Rust backend
interface DeviceInfo {
  id: string;
  name: string;
  hardware_type: 'Acoustic' | 'Special';
  driver_id: string;
}

interface ChannelRoute {
  Direct?: number;
  Mix?: number[];
}

interface ChannelMapping {
  physical_channels: number;
  virtual_channels: number;
  routing: ChannelRoute[];
}

interface Calibration {
  gain: number;
  offset: number;
}

interface RegisteredHardware {
  registration_id: string;
  device_id: string;
  hardware_name: string;
  driver_id: string;
  hardware_type: 'Acoustic' | 'Special';
  direction: 'Input' | 'Output';
  user_name: string;
  enabled: boolean;
  protocol?: 'ASIO' | 'CoreAudio' | 'ALSA' | 'WASAPI' | 'Jack';
  sample_rate: number;
  channels: number;
  channel_mapping: ChannelMapping;
  calibration: Calibration;
  max_voltage: number;
  notes: string;
}

export function HardwareConfig() {
  const [availableDevices, setAvailableDevices] = useState<DeviceInfo[]>([]);
  const [registeredHardware, setRegisteredHardware] = useState<RegisteredHardware[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);
  const [userName, setUserName] = useState('');
  const [direction, setDirection] = useState<'Input' | 'Output'>('Input');

  // Step 1: Update loadData to fetch from backend
  const loadData = async () => {
    setLoading(true);
    try {
      // Discover available devices
      const devices = await invoke<DeviceInfo[]>('discover_hardware');
      setAvailableDevices(devices);

      // Load registered hardware from backend
      const registered = await invoke<RegisteredHardware[]>('get_registered_devices');
      setRegisteredHardware(registered);

      // Mirror to localStorage for offline display
      localStorage.setItem('registered_hardware', JSON.stringify(registered));
    } catch (err) {
      console.error('Failed to load hardware:', err);

      // Fallback to localStorage if backend fails
      const saved = localStorage.getItem('registered_hardware');
      if (saved) {
        setRegisteredHardware(JSON.parse(saved));
      }
    } finally {
      setLoading(false);
    }
  };

  // Step 2: Update addHardware to use backend
  const addHardware = async (hw: RegisteredHardware) => {
    try {
      await invoke('register_device', { device: hw });

      // Update local state
      const updated = [...registeredHardware, hw];
      setRegisteredHardware(updated);

      // Mirror to localStorage
      localStorage.setItem('registered_hardware', JSON.stringify(updated));
    } catch (err) {
      console.error('Failed to register device:', err);
      alert(`Failed to register device: ${err}`);
    }
  };

  // Step 3: Update updateHardware to use backend
  const updateHardware = async (registrationId: string, updates: Partial<RegisteredHardware>) => {
    try {
      const device = registeredHardware.find(h => h.registration_id === registrationId);
      if (!device) return;

      const updated = { ...device, ...updates };
      await invoke('update_device', { registrationId, device: updated });

      // Update local state
      const newList = registeredHardware.map(h =>
        h.registration_id === registrationId ? updated : h
      );
      setRegisteredHardware(newList);

      // Mirror to localStorage
      localStorage.setItem('registered_hardware', JSON.stringify(newList));
    } catch (err) {
      console.error('Failed to update device:', err);
      alert(`Failed to update device: ${err}`);
    }
  };

  // Step 4: Update removeHardware to use backend
  const removeHardware = async (registrationId: string) => {
    try {
      await invoke('remove_device', { registrationId });

      // Update local state
      const updated = registeredHardware.filter(h => h.registration_id !== registrationId);
      setRegisteredHardware(updated);

      // Mirror to localStorage
      localStorage.setItem('registered_hardware', JSON.stringify(updated));
    } catch (err) {
      console.error('Failed to remove device:', err);
      alert(`Failed to remove device: ${err}`);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleAddDevice = async () => {
    if (!selectedDevice || !userName.trim()) {
      alert('Please select a device and provide a user name');
      return;
    }

    const newHardware: RegisteredHardware = {
      registration_id: `reg-${Date.now()}`,
      device_id: selectedDevice.id,
      hardware_name: selectedDevice.name,
      driver_id: selectedDevice.driver_id,
      hardware_type: selectedDevice.hardware_type,
      direction,
      user_name: userName.trim(),
      enabled: true,
      protocol: 'CoreAudio',
      sample_rate: 48000,
      channels: 2,
      channel_mapping: {
        physical_channels: 2,
        virtual_channels: 2,
        routing: [{ Direct: 0 }, { Direct: 1 }],
      },
      calibration: { gain: 1.0, offset: 0.0 },
      max_voltage: 0.0,
      notes: '',
    };

    await addHardware(newHardware);
    setShowAddForm(false);
    setSelectedDevice(null);
    setUserName('');
  };

  const toggleEnabled = async (registrationId: string) => {
    const device = registeredHardware.find(h => h.registration_id === registrationId);
    if (device) {
      await updateHardware(registrationId, { enabled: !device.enabled });
    }
  };

  if (loading) {
    return <div className="text-slate-400 p-5">Loading hardware configuration...</div>;
  }

  return (
    <div className="p-5">
      <div className="flex justify-between items-center mb-5">
        <h2 className="text-white text-2xl font-bold">Hardware Configuration</h2>
        <div className="flex gap-2">
          <Button onClick={loadData} variant="outline">Refresh</Button>
          <Button onClick={() => setShowAddForm(!showAddForm)}>
            {showAddForm ? 'Cancel' : 'Add Device'}
          </Button>
        </div>
      </div>

      {showAddForm && (
        <Card className="mb-5 p-4 bg-slate-800 border-slate-700">
          <h3 className="text-white text-lg font-semibold mb-3">Register New Device</h3>
          <div className="space-y-3">
            <div>
              <Label className="text-slate-300">Available Devices</Label>
              <select
                className="w-full bg-slate-700 text-white border border-slate-600 rounded px-3 py-2"
                value={selectedDevice?.id || ''}
                onChange={(e) => {
                  const device = availableDevices.find(d => d.id === e.target.value);
                  setSelectedDevice(device || null);
                }}
              >
                <option value="">Select a device...</option>
                {availableDevices.map(device => (
                  <option key={device.id} value={device.id}>
                    {device.name} ({device.hardware_type})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <Label className="text-slate-300">User Name</Label>
              <Input
                className="bg-slate-700 text-white border-slate-600"
                value={userName}
                onChange={(e) => setUserName(e.target.value)}
                placeholder="e.g., Main Microphone"
              />
            </div>
            <div>
              <Label className="text-slate-300">Direction</Label>
              <select
                className="w-full bg-slate-700 text-white border border-slate-600 rounded px-3 py-2"
                value={direction}
                onChange={(e) => setDirection(e.target.value as 'Input' | 'Output')}
              >
                <option value="Input">Input</option>
                <option value="Output">Output</option>
              </select>
            </div>
            <Button onClick={handleAddDevice} className="w-full">Register Device</Button>
          </div>
        </Card>
      )}

      <section className="mb-8">
        <h3 className="text-white text-xl font-semibold mb-3">Registered Devices</h3>
        {registeredHardware.length === 0 ? (
          <p className="text-slate-400">No devices registered yet</p>
        ) : (
          <div className="space-y-2">
            {registeredHardware.map(device => (
              <Card key={device.registration_id} className="p-4 bg-slate-800 border-slate-700">
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <h4 className="text-white font-semibold">{device.user_name}</h4>
                    <p className="text-slate-400 text-sm">
                      {device.hardware_name} • {device.direction} • {device.sample_rate}Hz • {device.channels}ch
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      variant={device.enabled ? 'default' : 'outline'}
                      size="sm"
                      onClick={() => toggleEnabled(device.registration_id)}
                    >
                      {device.enabled ? 'Enabled' : 'Disabled'}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => removeHardware(device.registration_id)}
                    >
                      Remove
                    </Button>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </section>

      <section>
        <h3 className="text-white text-xl font-semibold mb-3">Available Devices</h3>
        {availableDevices.length === 0 ? (
          <p className="text-slate-400">No devices discovered</p>
        ) : (
          <div className="space-y-2">
            {availableDevices.map(device => (
              <Card key={device.id} className="p-3 bg-slate-800 border-slate-700">
                <div className="flex items-center justify-between">
                  <div>
                    <span className="text-white">{device.name}</span>
                    <span className="text-slate-400 text-sm ml-2">({device.hardware_type})</span>
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

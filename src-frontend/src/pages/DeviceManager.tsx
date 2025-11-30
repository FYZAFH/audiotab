import { useState } from 'react';
import {
  useDiscoverDevices,
  useDeviceProfiles,
  useAddDeviceProfile,
  useDeleteDeviceProfile,
  type DeviceInfo,
  type DeviceProfile,
} from '../hooks/useDeviceManagement';
import { Button } from '../components/ui/button';
import { RefreshCw, Plus, Trash2, Settings } from 'lucide-react';

export function DeviceManager() {
  const [selectedProfile, setSelectedProfile] = useState<string | null>(null);

  const { data: profiles, isLoading: profilesLoading } = useDeviceProfiles();
  const { data: discoveredDevices, refetch: refetchDevices, isFetching: discovering } = useDiscoverDevices();
  const addProfile = useAddDeviceProfile();
  const deleteProfile = useDeleteDeviceProfile();

  const handleDiscoverDevices = () => {
    refetchDevices();
  };

  const handleAddDevice = (device: DeviceInfo) => {
    const newProfile: DeviceProfile = {
      id: `${device.driver_id}-${device.id}-${Date.now()}`,
      alias: device.name,
      driver_id: device.driver_id,
      device_id: device.id,
      config: {
        name: device.name,
        sample_rate: 48000,
        format: { type: 'F32' },
        buffer_size: 1024,
        channel_mapping: {
          physical_channels: 2,
          virtual_channels: 2,
          routing: [
            { Direct: 0 },
            { Direct: 1 },
          ],
        },
        calibration: {
          gain: 1.0,
          offset: 0.0,
        },
      },
      metadata: {
        description: '',
        tags: [],
        created_at: Math.floor(Date.now() / 1000),
        modified_at: Math.floor(Date.now() / 1000),
      },
    };

    addProfile.mutate(newProfile);
  };

  const handleDeleteProfile = (id: string) => {
    if (confirm('Delete this device profile?')) {
      deleteProfile.mutate(id);
    }
  };

  return (
    <div className="h-full flex flex-col bg-slate-900 text-white">
      {/* Header */}
      <div className="border-b border-slate-700 px-6 py-4">
        <h1 className="text-2xl font-semibold">Device Manager</h1>
        <p className="text-sm text-slate-400 mt-1">
          Configure audio devices and hardware profiles
        </p>
      </div>

      <div className="flex-1 grid grid-cols-2 gap-6 p-6 overflow-hidden">
        {/* Left Panel: Device Discovery */}
        <div className="bg-slate-800 rounded-lg border border-slate-700 flex flex-col">
          <div className="border-b border-slate-700 p-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">Available Devices</h2>
            <Button
              onClick={handleDiscoverDevices}
              disabled={discovering}
              variant="outline"
              size="sm"
            >
              <RefreshCw className={`h-4 w-4 mr-2 ${discovering ? 'animate-spin' : ''}`} />
              Discover
            </Button>
          </div>

          <div className="flex-1 overflow-auto p-4 space-y-2">
            {discoveredDevices?.map((device) => (
              <div
                key={`${device.driver_id}-${device.id}`}
                className="bg-slate-700 rounded p-3 flex items-center justify-between hover:bg-slate-600 transition-colors"
              >
                <div>
                  <div className="font-medium">{device.name}</div>
                  <div className="text-sm text-slate-400">
                    {device.driver_id} • {device.hardware_type}
                  </div>
                </div>
                <Button
                  onClick={() => handleAddDevice(device)}
                  variant="outline"
                  size="sm"
                >
                  <Plus className="h-4 w-4 mr-1" />
                  Add
                </Button>
              </div>
            ))}

            {!discoveredDevices && !discovering && (
              <div className="text-center text-slate-400 py-8">
                Click "Discover" to find available devices
              </div>
            )}
          </div>
        </div>

        {/* Right Panel: Configured Profiles */}
        <div className="bg-slate-800 rounded-lg border border-slate-700 flex flex-col">
          <div className="border-b border-slate-700 p-4">
            <h2 className="text-lg font-semibold">Device Profiles</h2>
          </div>

          <div className="flex-1 overflow-auto p-4 space-y-2">
            {profilesLoading && (
              <div className="text-center text-slate-400 py-8">Loading...</div>
            )}

            {profiles?.map((profile) => (
              <div
                key={profile.id}
                className={`bg-slate-700 rounded p-3 cursor-pointer transition-colors ${
                  selectedProfile === profile.id ? 'ring-2 ring-blue-500' : 'hover:bg-slate-600'
                }`}
                onClick={() => setSelectedProfile(profile.id)}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="font-medium">{profile.alias}</div>
                    <div className="text-sm text-slate-400 mt-1">
                      {profile.config.sample_rate / 1000}kHz • {profile.config.channel_mapping.virtual_channels}ch
                    </div>
                    <div className="text-xs text-slate-500 mt-1">
                      {profile.driver_id}
                    </div>
                  </div>

                  <div className="flex gap-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        // TODO: Open configuration dialog
                      }}
                    >
                      <Settings className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteProfile(profile.id);
                      }}
                    >
                      <Trash2 className="h-4 w-4 text-red-400" />
                    </Button>
                  </div>
                </div>
              </div>
            ))}

            {profiles?.length === 0 && !profilesLoading && (
              <div className="text-center text-slate-400 py-8">
                No device profiles configured
                <br />
                <span className="text-sm">Add devices from the left panel</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

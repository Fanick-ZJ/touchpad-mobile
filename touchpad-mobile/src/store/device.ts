import { defineStore } from "pinia";

export type Device = {
  name: string;
  fullName: string;
  address: string;
  loginPort: number;
  backendPort: number;
};

export const useDeviceStore = defineStore("device", {
  state: () => ({
    devices: [] as Device[],
    current_device: null as Device | null,
  }),
  actions: {
    addDevice(device: Device) {
      for (const existingDevice of this.devices) {
        if (existingDevice.address === device.address) {
          return;
        }
      }
      this.devices.push(device);
    },
    removeDevice(fullName: string) {
      this.devices = this.devices.filter(
        (device) => device.fullName !== fullName,
      );
    },
    setCurrentDevice(device: Device) {
      this.current_device = device;
    },
  },
});

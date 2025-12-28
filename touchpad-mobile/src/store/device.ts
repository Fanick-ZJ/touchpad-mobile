import { defineStore } from "pinia";

export type Device = {
  name: string;
  address: string;
  login_port: number;
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
    removeDevice(address: string) {
      this.devices = this.devices.filter(
        (device) => device.address !== address,
      );
    },
    setCurrentDevice(device: Device) {
      this.current_device = device;
    },
  },
});

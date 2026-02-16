import {
  startConnection,
  disconnectDevice,
  sendTouchPoints,
  sendTuneSetting,
} from "@/ipc/command";
import { DiscoverDevice, FrontTouchPoint, TuneSetting } from "@/ipc/types";
import { showToast } from "@bling-yshs/tauri-plugin-toast";
import { defineStore } from "pinia";

export class Device {
  constructor(
    public name: string,
    public fullName: string,
    public address: string,
    public loginPort: number,
    public backendPort: number,
  ) {}

  async connect() {
    startConnection(this)
      .then((success) => {
        if (success) {
          showToast("连接成功");
        } else {
          showToast("连接失败");
        }
      })
      .catch((error) => {
        console.error("连接设备时发生错误:", error);
        showToast("连接失败");
      });
  }

  async disconnect() {
    disconnectDevice(this);
  }
}

export const useDeviceStore = defineStore("device", {
  state: () => ({
    devices: [] as Device[],
    controledDevices: [] as Device[],
  }),
  actions: {
    addDevice(device: DiscoverDevice) {
      for (const existingDevice of this.devices) {
        if (existingDevice.address === device.address) {
          return;
        }
      }
      this.devices.push(
        new Device(
          device.name,
          device.fullName,
          device.address,
          device.loginPort,
          device.backendPort,
        ),
      );
    },
    removeDevice(fullName: string) {
      this.devices = this.devices.filter(
        (device) => device.fullName !== fullName,
      );
    },
    addControledDevice(device: Device) {
      if (!this.controledDevices.includes(device)) {
        this.controledDevices.push(device);
      }
    },
    removeControledDevice(device: Device) {
      this.controledDevices = this.controledDevices.filter((d) => d !== device);
    },
    isControledDevice(device: Device) {
      return this.controledDevices.includes(device);
    },
    sendTouchPoints(device: Device, points: FrontTouchPoint[]) {
      sendTouchPoints(device, points);
    },
    sendTouchPointsConnected(points: FrontTouchPoint[]) {
      for (const device of this.controledDevices) {
        sendTouchPoints(device, points);
      }
    },
    sendTuneSetting(setting: TuneSetting) {
      console.log(setting);
      for (const device of this.controledDevices) {
        sendTuneSetting(device, setting);
      }
    },
  },
});

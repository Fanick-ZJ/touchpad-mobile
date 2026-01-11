import { Device, useDeviceStore } from "@/store/device";
import { listen } from "@tauri-apps/api/event";
import { DiscoverDevice } from "./types";

const initListen = async () => {
  const deviceStore = useDeviceStore();
  await listen<DiscoverDevice>("found-device", (event) => {
    console.log("Discover event received:", event.payload);
    deviceStore.addDevice(event.payload);
  });

  await listen("device-login", (event) => {
    console.log("Device login event received:", event.payload);
    // deviceStore.updateDeviceStatus(event.payload, "connected");
  });

  await listen<string>("device-offline", (event) => {
    console.log("Device logout event received:", event.payload);
    deviceStore.removeDevice(event.payload);
    // deviceStore.updateDeviceStatus(event.payload, "disconnected");
  });
};

export { initListen };

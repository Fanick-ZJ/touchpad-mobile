import { Device, useDeviceStore } from "@/store/device";
import { listen } from "@tauri-apps/api/event";

const initListen = async () => {
  const deviceStore = useDeviceStore();
  await listen<Device>("found_device", (event) => {
    console.log("Discover event received:", event.payload);
    deviceStore.addDevice(event.payload);
  });
};

export { initListen };

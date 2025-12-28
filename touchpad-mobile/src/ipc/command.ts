import { Device } from "@/store/device";
import { invoke } from "@tauri-apps/api/core";

const startDiscoverService = async () => {
  await invoke("start_discover_service");
};

const startConnection = async (device: Device) => {
  await invoke("start_connection", { device });
};

export { startDiscoverService, startConnection };

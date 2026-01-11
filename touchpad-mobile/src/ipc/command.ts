import { Device } from "@/store/device";
import { invoke } from "@tauri-apps/api/core";

const startDiscoverService = async () => {
  await invoke("start_discover_service");
};

const startConnection = async (device: Device): Promise<Boolean> => {
  return await invoke("start_connection", { device });
};

const disconnectDevice = async (device: Device): Promise<Boolean> => {
  return await invoke("disconnect_device", { device });
};

export { startDiscoverService, startConnection, disconnectDevice };

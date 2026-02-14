import { Device } from "@/store/device";
import { invoke } from "@tauri-apps/api/core";
import { FrontTouchPoint } from "./types";

const startDiscoverService = async () => {
  await invoke("start_discover_service");
};

const startConnection = async (device: Device): Promise<Boolean> => {
  return await invoke("start_connection", { device });
};
startConnection;

const disconnectDevice = async (device: Device): Promise<Boolean> => {
  return await invoke("disconnect_device", { device });
};

const sendTouchPoints = async (
  device: Device,
  points: Array<FrontTouchPoint>,
): Promise<Boolean> => {
  return await invoke("send_touch_points", { device, touchPoints: points });
};

export {
  startDiscoverService,
  startConnection,
  disconnectDevice,
  sendTouchPoints,
};

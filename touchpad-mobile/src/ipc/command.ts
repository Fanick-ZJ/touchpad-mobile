import { invoke } from "@tauri-apps/api/core";

const startDiscoverService = async () => {
  await invoke("start_discover_service");
};

export { startDiscoverService };

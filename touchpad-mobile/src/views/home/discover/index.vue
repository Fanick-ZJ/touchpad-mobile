<script setup lang="ts">
import { onMounted } from "vue";
import { useDeviceStore } from "@/store/device";
import { startDiscoverService } from "@/ipc/command";
import DiscoverDevice from "./discover_device.vue";
import Page from "@/components/page.vue";
import { useRouter } from "vue-router";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";

const deviceStore = useDeviceStore();
const router = useRouter();

onMounted(async () => {
  await startDiscoverService();
  let permissionGranted = await isPermissionGranted();

  // If not we need to request it
  if (!permissionGranted) {
    const permission = await requestPermission();
    permissionGranted = permission === "granted";
  }
});
</script>

<template>
  <Page title="发现">
    <div class="flex flex-col grap-2">
      <template v-for="device in deviceStore.devices" :key="device.address">
        <DiscoverDevice :device="device"
      /></template>
    </div>
  </Page>
</template>

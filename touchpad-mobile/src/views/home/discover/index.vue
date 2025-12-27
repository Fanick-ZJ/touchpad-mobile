<script setup lang="ts">
import { onMounted } from "vue";
import { Device, useDeviceStore } from "@/store/device";
import { startDiscoverService } from "@/ipc/command";
import DiscoverDevice from "./discover_device.vue";
import Page from "@/components/page.vue";

const deviceStore = useDeviceStore();

onMounted(async () => {
    await startDiscoverService();
});

const deviceClick = (device: Device) => {
    console.log("Device clicked:", device);
};
</script>

<template>
    <Page title="Discover">
        <div class="flex flex-col grap-2">
            <template v-for="device in deviceStore.devices" :key="device.id">
                <DiscoverDevice :device="device" @click="deviceClick(device)"
            /></template>
        </div>
    </Page>
</template>

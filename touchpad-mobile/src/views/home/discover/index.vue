<script setup lang="ts">
import { onMounted } from "vue";
import { Device, useDeviceStore } from "@/store/device";
import { startDiscoverService, startConnection } from "@/ipc/command";
import DiscoverDevice from "./discover_device.vue";
import Page from "@/components/page.vue";
import { useRouter } from "vue-router";

const deviceStore = useDeviceStore();
const router = useRouter();

onMounted(async () => {
    await startDiscoverService();
});

const deviceClick = (device: Device) => {
    deviceStore.setCurrentDevice(device);
    // todo: 建立连接，跳转到控制页面
    startConnection(device);
    // router.push({ path: "/control" });
};
</script>

<template>
    <Page title="发现">
        <div class="flex flex-col grap-2">
            <template v-for="device in deviceStore.devices" :key="device.id">
                <DiscoverDevice :device="device" @click="deviceClick(device)"
            /></template>
        </div>
    </Page>
</template>

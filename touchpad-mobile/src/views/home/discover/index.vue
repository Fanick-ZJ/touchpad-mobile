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
</script>

<template>
    <Page title="Discover">
        <template v-for="i in 10">
            <template v-for="device in deviceStore.devices" :key="device.id">
                <DiscoverDevice :device="device" /> </template
        ></template>
    </Page>
</template>

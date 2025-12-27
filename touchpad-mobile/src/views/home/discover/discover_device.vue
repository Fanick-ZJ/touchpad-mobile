<script setup lang="ts">
import { ref, computed } from "vue";
import type { Device } from "@/store/device";
import { useDeviceStore } from "@/store/device";
const { device } = defineProps<{
    device: Device;
}>();

const subtitle = computed(() => {
    return "主机地址: " + device.address;
});
</script>

<template>
    <var-card ripple :title="device.name" :subtitle="subtitle" />
</template>

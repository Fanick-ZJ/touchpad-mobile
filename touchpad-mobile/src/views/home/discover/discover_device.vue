<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import type { Device } from "@/store/device";
import { useDeviceStore } from "@/store/device";
import type { Menu } from "@varlet/ui/types/menu";
const { device } = defineProps<{
  device: Device;
}>();

const cardRef = ref<HTMLElement | null>();
const deviceStore = useDeviceStore();
const menuRef = ref<Menu | null>(null);
const menuOffset = ref<[number, number]>([0, 0]);

onMounted(() => {
  console.log(menuRef.value);
});

const addressTitle = computed(() => {
  return "主机地址: " + device.address;
});
const isCurrentDevice = computed(() => {
  return deviceStore.isControledDevice(device);
});

const longTapHandler = (e: TouchEvent) => {
  menuRef.value?.open();
  const rect = cardRef.value?.getBoundingClientRect();
  const origin_x = rect?.left || 0;
  const origin_y = rect?.top || 0;
  menuOffset.value = [
    e.changedTouches[0].clientX - origin_x,
    e.changedTouches[0].clientY - origin_y,
  ];
};

const deviceClick = async () => {
  if (!deviceStore.isControledDevice(device)) {
    deviceStore.addControledDevice(device);
    await device.connect();
  }
};

const deviceDisconnect = async () => {
  if (deviceStore.isControledDevice(device)) {
    await device.disconnect();
    deviceStore.removeControledDevice(device);
  }
  menuRef.value?.close();
};

const emit = defineEmits<{
  (e: "click", device: Device): void;
}>();
</script>

<template>
  <div ref="cardRef">
    <var-card
      ripple
      class="relative"
      :title="device.name"
      @click="deviceClick"
      v-touch:longtap="longTapHandler"
    >
      <div class="flex flex-row justify-between w-full">
        <div class="text-base">
          {{ addressTitle }}
        </div>
        <var-icon v-if="isCurrentDevice" name="check" />
      </div>
      <var-menu
        :offset-x="`${menuOffset[0]}px`"
        :offset-y="`${menuOffset[1]}px`"
        class="absolute top-0 left-0"
        ref="menuRef"
      >
        <template #menu>
          <var-cell v-if="isCurrentDevice" @click="deviceDisconnect"
            >退出连接</var-cell
          >
          <var-cell>拉黑</var-cell>
        </template>
      </var-menu>
    </var-card>
  </div>
</template>

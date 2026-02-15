<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { TouchStatus, FrontTouchPoint } from "@/ipc/types";
import {
  Orientation,
  restoreOrientation,
  setOrientation,
} from "tauri-plugin-orientation-api";
import { useDeviceStore } from "@/store/device";
import { onBeforeRouteLeave } from "vue-router";

const controlRef = ref<HTMLElement | null>(null);

const deviceStore = useDeviceStore();

// 获取新增的触点
const getNewTouches = (event: TouchEvent) => {
  let new_touch = [];
  for (let i = 0; i < event.touches.length; i++) {
    let has = false;
    for (let j = 0; j < event.changedTouches.length; j++) {
      if (event.touches[i].identifier === event.changedTouches[j].identifier) {
        has = true;
        break;
      }
    }
    if (has) {
      new_touch.push(event.touches[i]);
    }
  }
  return new_touch;
};

// 获取移除的触点
const getRemovedTouches = (event: TouchEvent) => {
  let removed_touch = [];
  for (let i = 0; i < event.changedTouches.length; i++) {
    let has = false;
    for (let j = 0; j < event.touches.length; j++) {
      if (event.changedTouches[i].identifier === event.touches[j].identifier) {
        has = true;
        break;
      }
    }
    if (!has) {
      removed_touch.push(event.changedTouches[i]);
    }
  }
  return removed_touch;
};

// 获取移动的触点
const getMovedTouches = (event: TouchEvent) => {
  let moved_touches = [];
  for (let i = 0; i < event.touches.length; i++) {
    moved_touches.push(event.touches[i]);
  }
  return moved_touches;
};

const touches_to_front = (
  touches: Touch[],
  status: TouchStatus,
): FrontTouchPoint[] => {
  const front_touches: FrontTouchPoint[] = [];
  for (let i = 0; i < touches.length; i++) {
    const touch = touches[i];
    front_touches.push({
      tracking_id: touch.identifier,
      x: Math.floor(touch.clientX),
      y: Math.floor(touch.clientY),
      status: status,
    });
  }
  return front_touches;
};

onBeforeRouteLeave(() => {
  setOrientation({
    orientation: Orientation.Portrait,
    hideNavigationBar: false,
    hideStatusBar: false,
  });
});

onMounted(() => {
  // 全屏显示
  setOrientation({
    orientation: Orientation.SensorLandscape,
    hideNavigationBar: true,
    hideStatusBar: true,
  });
  if (controlRef.value) {
    controlRef.value.addEventListener("touchstart", (event) => {
      event.preventDefault();
      const new_touches = getNewTouches(event);
      const front_touches = touches_to_front(new_touches, TouchStatus.Add);
      deviceStore.sendTouchPointsConnected(front_touches);
    });
    controlRef.value.addEventListener("touchend", (event) => {
      event.preventDefault();
      const removed_touch = getRemovedTouches(event);
      const front_touches = touches_to_front(removed_touch, TouchStatus.Leave);
      deviceStore.sendTouchPointsConnected(front_touches);
    });
    controlRef.value.addEventListener("touchmove", (event) => {
      event.preventDefault();
      const moved_touches = getMovedTouches(event);
      const front_touches = touches_to_front(moved_touches, TouchStatus.Move);
      deviceStore.sendTouchPointsConnected(front_touches);
    });
    controlRef.value.addEventListener("touchcancel", (event) => {
      // event.preventDefault();
    });
  }
});
</script>

<template>
  <div
    ref="controlRef"
    class="w-screen h-screen flex flex-col items-center justify-center"
  >
    <div></div>
  </div>
</template>

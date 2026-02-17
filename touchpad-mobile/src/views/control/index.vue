<script setup lang="ts">
import { ref, onMounted, reactive, toRaw } from "vue";
import { TouchStatus, FrontTouchPoint, TuneSetting } from "@/ipc/types";
import { Orientation, setOrientation } from "tauri-plugin-orientation-api";
import { useDeviceStore } from "@/store/device";
import { onBeforeRouteLeave } from "vue-router";
import Background from "./components/background.vue";
import { usePersistenceStore } from "@/store/persistence";

const touchpadRef = ref<HTMLElement | null>(null);

const deviceStore = useDeviceStore();
const persistenceStore = usePersistenceStore();

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
  deviceStore.sendTuneSetting(persistenceStore.tune);
  // 注册触控事件（使用 passive 提高性能）
  if (touchpadRef.value) {
    touchpadRef.value.addEventListener(
      "touchstart",
      (event) => {
        const new_touches = getNewTouches(event);
        const front_touches = touches_to_front(new_touches, TouchStatus.Add);
        deviceStore.sendTouchPointsConnected(front_touches);
      },
      { passive: true },
    );
    touchpadRef.value.addEventListener(
      "touchend",
      (event) => {
        const removed_touch = getRemovedTouches(event);
        const front_touches = touches_to_front(
          removed_touch,
          TouchStatus.Leave,
        );
        deviceStore.sendTouchPointsConnected(front_touches);
      },
      { passive: true },
    );
    touchpadRef.value.addEventListener(
      "touchmove",
      (event) => {
        const moved_touches = getMovedTouches(event);
        const front_touches = touches_to_front(moved_touches, TouchStatus.Move);
        deviceStore.sendTouchPointsConnected(front_touches);
      },
      { passive: true },
    );
    touchpadRef.value.addEventListener(
      "touchcancel",
      (event) => {
        // 注销所有的触控事件
        const touches = getRemovedTouches(event);
        const front_touches = touches_to_front(touches, TouchStatus.Leave);
        deviceStore.sendTouchPointsConnected(front_touches);
      },
      { passive: true },
    );
  }
});

const showTuneDialog = ref(false);

let oldTuneData = reactive<TuneSetting>(
  structuredClone(toRaw(persistenceStore.tune)),
);
const openTuneHandler = () => {
  const tune = persistenceStore.tune;
  oldTuneData.invertX = tune.invertX;
  oldTuneData.invertY = tune.invertY;
  oldTuneData.sensitivity = tune.sensitivity;
};
const confirmTuneHandler = () => {
  const tune = persistenceStore.tune;
  tune.invertX = oldTuneData.invertX;
  tune.invertY = oldTuneData.invertY;
  tune.sensitivity = oldTuneData.sensitivity;
  persistenceStore.set("tune", tune);
  deviceStore.sendTuneSetting(tune);
};
</script>

<template>
  <div class="w-screen h-screen flex flex-col items-center justify-center">
    <div ref="touchpadRef" class="touchpad-area">
      <Background color="gray">
        <!-- 工具按钮 -->
        <var-fab type="primary">
          <var-button
            type="info"
            icon-container
            @click="showTuneDialog = !showTuneDialog"
          >
            <var-icon namespace="i" name="tune-variant" />
          </var-button>
        </var-fab>
      </Background>
    </div>
  </div>
  <var-dialog
    v-model:show="showTuneDialog"
    title="触控调节"
    @open="openTuneHandler"
    @confirm="confirmTuneHandler"
  >
    <div class="tune-content">
      <!-- 这里可以放任何自定义内容 -->
      <var-space direction="column" size="large">
        <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-3 items-center">
          <span>灵敏度</span>
          <var-slider
            v-model="oldTuneData.sensitivity"
            min="0.1"
            max="50"
            step="0.1"
          />
          <span>反转Y轴</span>
          <var-switch v-model="oldTuneData.invertY" />
          <span>反转X轴</span>
          <var-switch v-model="oldTuneData.invertX" />
        </div>
      </var-space>
    </div>
  </var-dialog>
</template>

<style scoped>
.touchpad-area {
  width: 100%;
  height: 100%;
  touch-action: none; /* 禁止所有默认触摸行为，包括滚动、缩放等 */
}
</style>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { TouchStatus, TouchEventTypes } from "@/ipc/types";

const controlRef = ref<HTMLElement | null>(null);
const touchStatus = ref<TouchStatus | null>(null);
const touchType = ref<TouchEventTypes | null>(null);

// 获取新增的触点
const getNewTouches = (event: TouchEvent) => {
    let new_touch = [];
    for (let i = 0; i < event.touches.length; i++) {
        let has = false;
        for (let j = 0; j < event.changedTouches.length; j++) {
            if (
                event.touches[i].identifier ===
                event.changedTouches[j].identifier
            ) {
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
            if (
                event.changedTouches[i].identifier ===
                event.touches[j].identifier
            ) {
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

onMounted(() => {
    if (controlRef.value) {
        controlRef.value.addEventListener("touchstart", (event) => {
            event.preventDefault();
            let new_touch = getNewTouches(event);
            if (
                touchStatus.value === null ||
                touchStatus.value === TouchStatus.ENDED
            ) {
                touchStatus.value = TouchStatus.STARTED;
                touchType.value = TouchEventTypes.JOINED;
            } else if (touchStatus.value === TouchStatus.STARTED) {
            }
        });
        controlRef.value.addEventListener("touchend", (event) => {
            event.preventDefault();
            const removed_touch = getRemovedTouches(event);
            if (touchStatus.value === TouchStatus.STARTED) {
                touchStatus.value = TouchStatus.ENDED;
            }
        });
        controlRef.value.addEventListener("touchmove", (event) => {
            event.preventDefault();
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
        <div>
            <p>Touch Type: {{ touchType }}</p>
            <p>Touch Status: {{ touchStatus }}</p>
        </div>
    </div>
</template>

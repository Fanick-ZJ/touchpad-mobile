<script setup lang="ts">
import { ref, watch } from "vue";
import { useRouter, useRoute } from "vue-router";
const active = ref("discover");

const router = useRouter();
watch(
    () => active.value,
    (newValue) => {
        console.log(`Active tab changed to: ${newValue}`);
        if (newValue === "control") {
            router.push("/control");
        } else if (newValue === "settings") {
            router.push("/home/settings");
        } else if (newValue === "discover") {
            router.push("/home/discover");
        }
    },
);

watch(
    () => router.currentRoute.value.path,
    (newValue) => {
        console.log(`Route path changed to: ${newValue}`);
        if (newValue === "/home/discover") {
            active.value = "discover";
        } else if (newValue === "/control") {
            active.value = "control";
        } else if (newValue === "/home/settings") {
            active.value = "settings";
        }
    },
);
</script>

<template>
    <div class="h-full w-full flex flex-col">
        <RouterView class="flex-1 overflow-hidden" />
        <var-bottom-navigation variant v-model:active="active">
            <var-bottom-navigation-item
                label="发现"
                name="discover"
                icon="magnify"
                style="--ripple-color: transparent"
            ></var-bottom-navigation-item>
            <var-bottom-navigation-item
                label="控制"
                name="control"
                icon="i-gesture-tap-button"
                style="--ripple-color: transparent"
            >
                <template #icon>
                    <var-icon
                        size="24px"
                        namespace="i"
                        name="gesture-tap-button"
                    />
                </template>
            </var-bottom-navigation-item>
            <var-bottom-navigation-item
                label="设置"
                name="settings"
                icon="setting"
                style="--ripple-color: transparent"
            >
                <template #icon>
                    <var-icon
                        size="24px"
                        namespace="i"
                        name="cogs"
                    /> </template
            ></var-bottom-navigation-item>
        </var-bottom-navigation>
    </div>
</template>

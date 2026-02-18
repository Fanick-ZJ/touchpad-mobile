<script setup lang="ts">
import { ref, watch } from "vue";
import { useRouter } from "vue-router";
const active = ref("discover");

const router = useRouter();
watch(
  () => active.value,
  (newValue) => {
    console.log(`Active tab changed to: ${newValue}`);
    if (newValue === "control") {
      router.push("/control");
    } else if (newValue === "about") {
      router.push("/home/about");
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
    } else if (newValue === "/home/about") {
      active.value = "about";
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
        namespace="i"
        icon="gesture-tap-button"
        style="--ripple-color: transparent"
      >
        <template #icon>
          <var-icon size="24px" namespace="i" name="gesture-tap-button" />
        </template>
      </var-bottom-navigation-item>
      <var-bottom-navigation-item
        label="关于"
        name="about"
        namespace="i"
        icon="information-outline"
        style="--ripple-color: transparent"
      >
        <template #icon>
          <var-icon
            size="24px"
            namespace="i"
            name="information-outline"
          /> </template
      ></var-bottom-navigation-item>
    </var-bottom-navigation>
  </div>
</template>

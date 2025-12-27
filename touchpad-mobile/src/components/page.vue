<script setup lang="ts">
defineProps({
    title: String,
});
</script>

<template>
    <div class="page bg-gray-100 max-h-full h-full flex flex-col">
        <var-app-bar title-position="left" :elevation="false" safe-area-top>
            <h1 class="font-bold text-xl">{{ title }}</h1>
            <template #header-left>
                <slot name="left"></slot>
            </template>
            <template #header-right>
                <slot name="right"></slot>
            </template>
            <template #header-center>
                <slot name="center"></slot>
            </template>
        </var-app-bar>
        <div
            class="content flex-1 flex flex-col gap-2"
            direction="column"
            size="large"
        >
            <slot></slot>
        </div>
    </div>
</template>

<style scoped>
.content {
    padding: 5px 15px 15px;
}
</style>

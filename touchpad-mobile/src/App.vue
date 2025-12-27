<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
</script>

<template>
    <main class="container select-none flex flex-col">
        <RouterView />
    </main>
</template>

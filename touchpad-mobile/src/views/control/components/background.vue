<script setup lang="ts">
const props = defineProps({
  color: {
    type: String,
    default: "#6B7280",
  },
  dotSpacing: {
    type: Number,
    default: 20,
  },
  dotRadius: {
    type: Number,
    default: 2,
  },
});
</script>

<template>
  <div class="relative w-full h-full">
    <svg
      xmlns="http://www.w3.org/2000/svg"
      class="w-full h-full"
      preserveAspectRatio="none"
    >
      <defs>
        <!-- 定义点阵图案 -->
        <pattern
          id="dotPattern"
          x="0"
          y="0"
          :width="props.dotSpacing"
          :height="props.dotSpacing"
          patternUnits="userSpaceOnUse"
        >
          <circle
            :cx="props.dotSpacing / 2"
            :cy="props.dotSpacing / 2"
            :r="props.dotRadius"
            :fill="props.color"
            opacity="0.3"
          />
        </pattern>
      </defs>

      <!-- 点阵填充（留一点边距） -->
      <rect x="0" y="0" width="100%" height="100%" fill="url(#dotPattern)" />
    </svg>
    <div class="absolute inset-0">
      <slot />
    </div>
  </div>
</template>

<style scoped>
/* 确保SVG填满容器 */
svg {
  display: block;
}
</style>

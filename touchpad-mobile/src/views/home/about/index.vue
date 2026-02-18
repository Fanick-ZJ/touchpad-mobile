<script setup lang="ts">
import Page from "@/components/page.vue";

const appVersion = "0.1.0";
const appDescription = "一款跨平台的触摸板远程控制应用";

// 功能特性
const features = [
  { icon: "wifi", title: "局域网发现", description: "自动发现同一网络下的设备" },
  { icon: "gesture-tap-button", title: "多点触控", description: "支持多点触控和手势操作" },
  { icon: "cog", title: "灵敏度调节", description: "自定义触控灵敏度设置" },
  { icon: "fullscreen", title: "全屏控制", description: "沉浸式控制体验" },
];
</script>

<template>
  <Page title="关于">
    <div class="about-container">
      <!-- 应用图标和名称 -->
      <div class="app-header">
        <img src="/icon.png" alt="Touchpad Mobile" class="app-logo" />
        <h1 class="app-name">Touchpad Mobile</h1>
        <p class="app-version">v{{ appVersion }}</p>
        <p class="app-description">{{ appDescription }}</p>
      </div>

      <!-- 功能特性 -->
      <div class="section">
        <h3 class="section-title">功能特性</h3>
        <div class="features-grid">
          <div
            class="feature-item"
            v-for="(feature, index) in features"
            :key="index"
          >
            <div class="feature-icon">
              <var-icon namespace="i" :name="feature.icon" size="28px" />
            </div>
            <div class="feature-content">
              <h4 class="feature-title">{{ feature.title }}</h4>
              <p class="feature-description">{{ feature.description }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- 版权信息 -->
      <div class="footer">
        <p class="copyright">© 2025 Touchpad Mobile</p>
      </div>
    </div>
  </Page>
</template>

<style scoped>
.about-container {
  padding: 16px;
  padding-bottom: 32px;
}

.app-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 32px 16px;
  text-align: center;
}

.app-logo {
  width: 96px;
  height: 96px;
  border-radius: 22px;
  margin-bottom: 16px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.app-name {
  font-size: 24px;
  font-weight: 600;
  color: var(--varlet-list-title-color);
  margin: 0 0 8px 0;
}

.app-version {
  font-size: 14px;
  color: var(--varlet-list-subtitle-color);
  margin: 0 0 12px 0;
  opacity: 0.8;
}

.app-description {
  font-size: 14px;
  color: var(--varlet-list-subtitle-color);
  max-width: 280px;
  line-height: 1.6;
  margin: 0;
}

.section {
  margin-top: 32px;
}

.section-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--varlet-list-title-color);
  margin: 0 0 16px 0;
  padding-left: 4px;
}

.features-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px 12px;
}

.feature-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 20px 12px;
  border-radius: 12px;
  transition: transform 0.2s;
  min-height: 110px;
}

.feature-item:active {
  transform: scale(0.96);
}

.feature-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  background: linear-gradient(135deg, #4a90e2 0%, #357abd 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 12px;
  flex-shrink: 0;
}

.feature-icon :deep(.var-icon) {
  color: white;
}

.feature-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  gap: 2px;
  width: 100%;
}

.feature-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--varlet-list-title-color);
  margin: 0;
  word-wrap: break-word;
}

.feature-description {
  font-size: 11px;
  color: var(--varlet-list-subtitle-color);
  line-height: 1.3;
  margin: 0;
  word-wrap: break-word;
}

.footer {
  text-align: center;
  padding: 32px 16px 16px;
  margin-top: 32px;
}

.copyright {
  font-size: 12px;
  color: var(--varlet-list-subtitle-color);
  margin: 0;
  opacity: 0.6;
}
</style>

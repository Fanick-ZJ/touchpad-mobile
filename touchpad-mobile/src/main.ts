import App from "./App.vue";
import { createApp } from "vue";
import { router } from "@/router/index";
import { initListen } from "@/ipc/listen";
import { createPinia } from "pinia";
import Varlet from "@varlet/ui";
import "@varlet/ui/es/style";
import "virtual-icons";
import Vue3TouchEvents, {
  type Vue3TouchEventsOptions,
} from "vue3-touch-events";
import { usePersistenceStore } from "@/store/persistence";

const pinia = createPinia();
const app = createApp(App);

app.use(router);
app.use(pinia);
app.use(Varlet);
app.use<Vue3TouchEventsOptions>(Vue3TouchEvents, {
  disableClick: false,
  // any other global options...
});

// 在挂载应用前初始化 store
const initApp = async () => {
  try {
    const persistenceStore = usePersistenceStore();
    await persistenceStore.ensureInitialized();
    console.log("Persistence store initialized successfully");
  } catch (error) {
    console.error("Failed to initialize persistence store:", error);
  }

  // 挂载应用
  app.mount("#app");

  // 初始化监听器
  initListen();
};

initApp();

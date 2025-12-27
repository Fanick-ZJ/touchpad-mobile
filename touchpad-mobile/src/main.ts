import App from "./App.vue";
import { createApp } from "vue";
import { router } from "@/router/index";
import { initListen } from "@/ipc/listen";
import { createPinia } from "pinia";
import Varlet from "@varlet/ui";
import "@varlet/ui/es/style";
import "virtual-icons";

const pinia = createPinia();
const app = createApp(App);

app.use(router);
app.use(pinia);
app.use(Varlet);
app.mount("#app");
initListen();

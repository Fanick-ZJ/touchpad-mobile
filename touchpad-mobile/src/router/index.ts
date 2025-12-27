import { createRouter, createWebHistory } from "vue-router";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      redirect: "/home/discover",
    },
    {
      path: "/home",
      redirect: "/home/discover",
      component: () => import("@/views/home/index.vue"),
      children: [
        {
          name: "discover",
          path: "/home/discover",
          component: () => import("@/views/home/discover/index.vue"),
        },
        {
          name: "settings",
          path: "/home/settings",
          component: () => import("@/views/home/settings/index.vue"),
        },
      ],
    },
    {
      path: "/control",
      component: () => import("@/views/control/index.vue"),
    },
  ],
});

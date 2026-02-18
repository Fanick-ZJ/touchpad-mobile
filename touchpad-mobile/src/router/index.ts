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
          name: "about",
          path: "/home/about",
          component: () => import("@/views/home/about/index.vue"),
        },
      ],
    },
    {
      path: "/control",
      component: () => import("@/views/control/index.vue"),
    },
  ],
});

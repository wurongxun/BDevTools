import { createRouter, createWebHistory } from "vue-router";
// 开启历史模式
// vue2中使用的mode：history 实现
const routerHistory = createWebHistory();
const router = createRouter({
  history: routerHistory,
  routes: [
    {
      path: "/",
      redirect: "/MainWindows/Note",
    },
    {
      path: "/MainWindows",
      component: () => import("../views/MainWindows.vue"),
      children: [
        {
          // 当 /user/:id/profile 匹配成功
          // UserProfile 将被渲染到 User 的 <router-view> 内部
          path: "Note",
          component: () => import("../views/main_windows/NoteBookEdit.vue"),
        },
      ],
    },
    {
      path: "/Windows/About",
      component: () => import("../views/windows/AppAbout.vue"),
    },
    {
      path: "/Windows/Loding",
      component: () => import("../views/loding_windows/AppLoding.vue"),
    },
  ],
});
export default router2;

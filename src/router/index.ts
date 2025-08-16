// Composables
import { nextTick } from "vue";
import {
  HistoryState,
  RouteRecordRaw,
  createRouter,
  createWebHashHistory,
} from "vue-router";
//import { Layout } from "@/utils/routerHelper";
const EL_SCROLL = ".v-main .mainScroll";

declare module "vue-router" {
  interface RouteMeta {
    transition?: string;
    savedPosition: ScrollToOptions;
  }
  interface HistoryState {
    forward: string | null;
    back: string | null;
    current: string | null;
    scroll: ScrollToOptions;
    //基于单个元素和路径的滚动配置
    scroll2: {
      [index: string]: ScrollToOptions;
    } | null;
    [index: string]: ScrollToOptions;
  }
}

const routes = <Readonly<RouteRecordRaw[]>>[
  {
    path: "/",
    redirect: "/AppMainView/MainView",
  },
  {
    path: "/MainWindows",
    component: () => import("@/views/common/Common.vue"),
    children: [
      {
        // 当 /user/:id/profile 匹配成功
        // UserProfile 将被渲染到 User 的 <router-view> 内部
        path: "NoteBook",
        name: "NoteBook",
        component: () => import("@/views/utilsView/notebook/NoteBookEdit.vue"),
      },
       {
        // 当 /user/:id/profile 匹配成功
        // UserProfile 将被渲染到 User 的 <router-view> 内部
        path: "Calendar",
        name: "Calendar",
        component: () => import("@/views/utilsView/calendar/Calendar.vue"),
      },
    ],
  },
  {
    path: "/AppMainView",
    component: () => import("@/views/appMainWindows.vue"),
    children: [
      {
        // 当 /user/:id/profile 匹配成功
        // UserProfile 将被渲染到 User 的 <router-view> 内部
        path: "MainView",
        component: () => import("@/views/mainView/mainView.vue"),
      },
    ],
  },
  ,
  {
    path: "/App",
    component: () => import("@/views/common/Common.vue"),
    children: [
      {
        path: "AppSetting",
        name: "AppSetting",
        component: () => import("@/views/app/AppSetting.vue"),
      },
      {
        path: "AppAbout",
        name: "AppAbout",
        component: () => import("@/views/app/AppAbout.vue"),
      },
     {
        path: "AppHelp",
        name: "AppHelp",
        component: () => import("@/views/app/AppHelp.vue"),
      },
    ],
  },
  {
    path: "/Windows/Loding",
    component: () => import("@/views/lodingWindows/AppLoding.vue"),
  },
];
// ,
// {
//   path: "/Business",
//   component: Layout,
//   children: [
//     {
//       path: "Ssh",
//       name: "Ssh",
//       component: () => import("@/views/element/Business/ssh/index.vue"),
//     },
//   ],
// },
//const history = createWebHashHistory(process.env.BASE_URL);
const history = createWebHashHistory("/");

const router = createRouter({
  history,
  routes,
});

const scrollState2 = history.state.scroll2 ?? {};

router.beforeEach((to, from) => {
  // 用户在主页点击首页时自动返回，防止有重复的历史记录。
  if (to.path === "/index/home" && to.path === history.state.back) {
    history.go(-1);
    console.warn(
      `backing to ${to.path} because history.state.back=${history.state.back}, state may lost.`
    );
    return false;
  }
  // TODO: 使用更好的方法实现
  const state = {
    ...history.state,
    scroll2: scrollState2,
  } as HistoryState;
  // 当前有一个bug，通过导航按钮切换的页面无法监听,因为current可能不等于from。这可能导致滚动状态无法被保存。此处使用内存泄漏掩盖部分bug

  const scrollData = {
    top: document.querySelector(EL_SCROLL)?.scrollTop ?? 0,
    left: document.querySelector(EL_SCROLL)?.scrollLeft ?? 0,
  };
  scrollState2[from.fullPath] = scrollData;
  console.debug("Saving scrollData:", scrollData);
  console.debug("Saving state", state);
  if (history.state.current !== from.fullPath) {
    console.warn(
      "history.state.current !== from.fullPath, state maybe not to be saved."
    );
  }
  window.history.replaceState(state, document.title);
  return true;
});

router.afterEach((to, from) => {
  if (from.path !== "/") {
    const name = history.state.forward
      ? "scroll-x-transition"
      : "scroll-x-reverse-transition";
    to.meta.transition = name;
    from.meta.transition = name;
  } else {
    const name = "fade-transition";
    to.meta.transition = name;
    from.meta.transition = name;
  }

  const state = window.history.state as HistoryState;
  if (state.scroll2 && state.current) {
    nextTick(() => {
      //有动画，所以要选择第最后一个元素
      if (state.scroll2 && state.current) {
        const elements = document.querySelectorAll(EL_SCROLL);
        elements[elements.length - 1]?.scrollTo(state.scroll2[state.current]);
      }
    });
  }
});

export default router;

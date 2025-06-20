import router from "@/router";
//import type { SshSubMenu } from 'vue-router'
//import { getRawRoute } from "@/utils/routerHelper";
import { defineStore } from "pinia";
import { store } from "../index";
import { findIndex } from "@/utils";
import { SshSubMenu, SshLinkInfoTree } from "@/entity/entitys";

export interface TagsViewState {
  visitedViews: SshSubMenu[];
  cachedViews: Set<string>;
  title: string;
  activeTag: SshSubMenu;
  sshLinkInfoTree: SshLinkInfoTree[];
}

export const useTagsViewStore = defineStore("SshtagsView", {
  state: (): TagsViewState => ({
    visitedViews: new Array<SshSubMenu>(),
    cachedViews: new Set(),
    title: "",
    activeTag: new SshSubMenu(),
    sshLinkInfoTree: [],
  }),
  getters: {
    getVisitedViews(): SshSubMenu[] {
      return this.visitedViews;
    },
    getCachedViews(): string[] {
      return Array.from(this.cachedViews);
    },
    getTitle(): string {
      return this.title;
    },
    getActiveTag(): SshSubMenu {
      return this.activeTag;
    },
    getSshLinkInfoTree(): SshLinkInfoTree[] {
      return this.sshLinkInfoTree;
    },
  },
  actions: {
    // 新增缓存和tag
    addView(view: SshSubMenu): void {
      this.addVisitedView(view);
      this.addCachedView();
    },
    // 新增tag
    addVisitedView(view: SshSubMenu) {
      console.log("addVisitedView view.id:", view.id);
      if (
        this.visitedViews.some((v) => {
          console.log("view.id:", view.id + " v.id: " + v.id);
          return v.id === view.id;
        })
      )
        return;
      //if (view.meta?.noTagsView) return
      this.visitedViews.push(
        Object.assign({}, view, {
          // title: view.meta?.title || 'no-name'
          title: view.id || "no-name",
        })
      );
    },
    // 新增缓存
    addCachedView() {
      const cacheMap: Set<string> = new Set();
      for (const v of this.visitedViews) {
        const item = v;
        // const needCache = !item.meta?.noCache
        // if (!needCache) {
        //   continue
        // }
        const name = item.name as string;
        cacheMap.add(name);
      }
      if (
        Array.from(this.cachedViews).sort().toString() ===
        Array.from(cacheMap).sort().toString()
      )
        return;
      this.cachedViews = cacheMap;
    },
    // 删除某个
    delView(view: SshSubMenu) {
      this.delVisitedView(view);
      this.delCachedView();
    },
    // 删除tag
    delVisitedView(view: SshSubMenu) {
      for (const [i, v] of this.visitedViews.entries()) {
        if (v.id === view.id) {
          this.visitedViews.splice(i, 1);
          break;
        }
      }
    },
    // 删除缓存
    delCachedView() {
      const route = router.currentRoute.value;
      const index = findIndex<string>(
        this.getCachedViews,
        (v) => v === route.name
      );
      if (index > -1) {
        this.cachedViews.delete(this.getCachedViews[index]);
      }
    },
    // 删除所有缓存和tag
    delAllViews() {
      this.delAllVisitedViews();
      this.delCachedView();
    },
    // 删除所有tag
    delAllVisitedViews() {
      // const affixTags = this.visitedViews.filter((tag) => tag.meta.affix)
      this.visitedViews = [];
    },
    // 删除其他
    delOthersViews(view: SshSubMenu) {
      this.delOthersVisitedViews(view);
      this.addCachedView();
    },
    // 删除其他tag
    delOthersVisitedViews(view: SshSubMenu) {
      this.visitedViews = this.visitedViews.filter((v) => {
        return v.affix || v.id === view.id;
      });
    },
    // 删除左侧
    delLeftViews(view: SshSubMenu) {
      const index = findIndex<SshSubMenu>(
        this.visitedViews,
        (v) => v.path === view.id
      );
      if (index > -1) {
        this.visitedViews = this.visitedViews.filter((v, i) => {
          return v?.affix || v.id === view.id || i > index;
        });
        this.addCachedView();
      }
    },
    // 删除右侧
    delRightViews(view: SshSubMenu) {
      const index = findIndex<SshSubMenu>(
        this.visitedViews,
        (v) => v.path === view.id
      );
      if (index > -1) {
        this.visitedViews = this.visitedViews.filter((v, i) => {
          return v?.affix || v.id === view.id || i < index;
        });
        this.addCachedView();
      }
    },
    updateVisitedView(view: SshSubMenu) {
      for (let v of this.visitedViews) {
        if (v.id === view.id) {
          v = Object.assign(v, view);
          break;
        }
      }
    },
    setTitle(title: string) {
      this.title = title;
    },
    setActiviteTag(tag: SshSubMenu) {
      this.activeTag = tag;
    },
    setSshLinkInfoTree(sshLinkInfoTree: SshLinkInfoTree) {
      this.sshLinkInfoTree.push(sshLinkInfoTree);
    },
    ClearSshLinkInfoTree() {
      this.sshLinkInfoTree = [];
    },
  },
  persist: false,
});

export const useTagsViewStoreWithOut = () => {
  return useTagsViewStore(store);
};

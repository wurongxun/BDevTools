import { MaybeRef, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";

interface NavPage {
  title: MaybeRef<string>;
  icon: string;
  activeIcon: string;
  name: string;
  disabled?: boolean;
  showWinMin?: boolean;
  showTitle?: boolean;
  showWinChange?: boolean;
  showWinHide?: boolean;
}

export function useHomeNavigation() {
  const { t } = useI18n();
  const route = useRoute();
  const router = useRouter();

  const pages = computed<NavPage[]>(() => {
    return [
      {
        title: t("home.name"),
        icon: "mdi-home-variant-outline",
        activeIcon: "mdi-home-variant",
        name: "Home",
      },
      {
        title: t("category.name", 2),
        icon: "mdi-apps",
        activeIcon: "mdi-apps",
        name: "Categories",
      },
      {
        title: t("Notebook.name", 2),
        icon: "mdi-apps",
        activeIcon: "mdi-apps",
        name: "NoteBook",
      },
      {
        title: t("ssh.SSH"),
        icon: "mdi-ssh",
        activeIcon: "mdi-ssh",
        name: "SecureShell",
      },
      {
        title: t("update.name"),
        icon: "mdi-update",
        activeIcon: "mdi-update",
        name: "Update",
        disabled: true,
      },
      {
        title: t("me.name"),
        icon: "mdi-account-outline",
        activeIcon: "mdi-account",
        name: "Me",
      },
      {
        title: t("settings.name"),
        icon: "mdi-setting",
        activeIcon: "mdi-setting",
        name: "AppSetting",
        showWinMin: true,
        showTitle: true,
        showWinChange: true,
        showWinHide: false,
      },
      {
        title: t("ssh.name"),
        icon: "mdi-ssh",
        activeIcon: "mdi-ssh",
        name: "Ssh",
        showWinMin: true,
        showTitle: true,
        showWinChange: true,
        showWinHide: false,
      },
      {
        title: t("help.name"),
        icon: "mdi-help",
        activeIcon: "mdi-help",
        name: "Help",
        showWinMin: true,
        showTitle: true,
        showWinChange: true,
        showWinHide: false,
      },
      {
        title: t("calendar.name"),
        icon: "mdi-calendar",
        activeIcon: "mdi-calendar",
        name: "Calendar",
        showWinMin: true,
        showTitle: true,
        showWinChange: true,
        showWinHide: false,
      },
    ];
  });
  const activePagePosition = computed(() =>
    route.name
      ? pages.value.findIndex((page) => page.name === route.name)
      : null
  );

  const isInMainView = computed(() => activePagePosition.value !== -1);

  const isBackOtherPage = computed<boolean>(() => {
    route.path; //确保路由刷新时重新调用该函数
    return !!(
      router.options.history.state.back &&
      router.options.history.state.back !== "/index/home"
    );
  });
  return { pages, activePagePosition, isInMainView, isBackOtherPage };
}

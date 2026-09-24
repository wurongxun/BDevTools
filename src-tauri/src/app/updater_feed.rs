//! 更新清单（仓库根 `latest.json`）与 update 端点配置的自检。
//!
//! 清单由发行方发版后更新、随仓库一起推送；应用运行时按 `tauri.conf.json` 里
//! `plugins.updater.endpoints` 指向的 **Gitee raw** 地址拉取它。
//!
//! 发版侧由 `scripts/release.mjs` 负责：升版本 → `tauri build`（NSIS，自动签名）→
//! 本地校验签名 → 上传 Gitee Release → 把清单写回仓库。手改清单容易漏掉签名与版本同步，
//! 所以这里再加一道「清单必须能被插件的反序列化器吃下」的自检兜底。
//!
//! 这两处一旦写错，`tauri-plugin-updater` 只会回一句
//! `Could not fetch a valid release JSON from the remote`（`Error::ReleaseNotFound`）——
//! 网络不通、状态码非 2xx、JSON 解析失败、字段缺失，四件事被折叠成同一个字符串，
//! 在应用里只表现为一行 `console.error`，很难定位。
//!
//! 所以这里用**插件自己的反序列化器**（`RemoteRelease`）把仓库里的清单过一遍，
//! 把这类问题拦在测试里。三个真实踩过的坑：
//! - 端点写成 Gitee 的 `blob/`（网页地址）→ 返回 HTML，永远解析失败，必须是 `raw/`；
//! - `"url": ""` 这种空占位 → `Url` 反序列化失败，**整份清单**失效，哪怕当前平台条目没问题；
//! - 清单里缺当前平台条目 → 插件在比较版本**之前**就要取地址，直接报 ReleaseNotFound。

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tauri_plugin_updater::{RemoteRelease, RemoteReleaseInner};

    /// 仓库根的更新清单（与端点指向的远端文件同源）
    fn release_feed() -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../latest.json");
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读不到更新清单 {}: {e}", path.display()))
    }

    /// `tauri.conf.json`
    fn app_conf() -> serde_json::Value {
        serde_json::from_str(include_str!("../../tauri.conf.json"))
            .expect("tauri.conf.json 不是合法 JSON")
    }

    /// 应用版本：updater 拿它和清单里的 version 比较
    fn app_version() -> String {
        app_conf()
            .get("version")
            .and_then(|v| v.as_str())
            .expect("tauri.conf.json 缺少 version")
            .to_string()
    }

    /// 已配置的 update 端点
    fn endpoints() -> Vec<String> {
        app_conf()
            .pointer("/plugins/updater/endpoints")
            .and_then(|v| v.as_array())
            .expect("tauri.conf.json 缺少 plugins.updater.endpoints")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    }

    fn feed() -> RemoteRelease {
        serde_json::from_str(&release_feed()).expect(
            "latest.json 不是合法的 updater 清单（运行时会被判成 \
             \"Could not fetch a valid release JSON from the remote\"）",
        )
    }

    /// 插件实际查找的平台 key：优先 `{os}-{arch}-{installer}`，其次 `{os}-{arch}`
    /// （见 tauri-plugin-updater 的 `get_urls`）
    fn plugin_target_keys() -> Vec<String> {
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            other => other,
        };
        let arch = std::env::consts::ARCH;
        vec![format!("{os}-{arch}-msi"), format!("{os}-{arch}")]
    }

    #[test]
    fn updater_endpoint_is_a_raw_json_url() {
        let endpoints = endpoints();
        assert!(!endpoints.is_empty(), "没有配置任何 update 端点");
        for url in &endpoints {
            // Gitee 的 `blob/` 是网页地址（返回 HTML），必须用 `raw/` 才是文件内容
            assert!(
                url.contains("/raw/"),
                "端点 {url} 不是 raw 直链：Gitee 的 blob/ 地址返回 HTML 页面，\
                 updater 只会报「Could not fetch a valid release JSON from the remote」"
            );
            assert!(
                url.ends_with(".json"),
                "端点 {url} 不以 .json 结尾，确认它指向的是更新清单"
            );
        }
    }

    #[test]
    fn release_feed_matches_updater_schema() {
        let release = feed();

        // 清单版本必须等于当前应用版本：占位清单若写高了，check() 会提示一个
        // 签名/地址还没准备好的「更新」；写低了则说明忘了随发版更新
        assert_eq!(
            release.version.to_string(),
            app_version(),
            "清单 version 与 tauri.conf.json 的 version 不一致"
        );

        // check() 在比较版本之前就要按平台取地址，缺条目同样报 ReleaseNotFound
        let keys = plugin_target_keys();
        let hit = keys.iter().find(|k| release.download_url(k).is_ok());
        assert!(
            hit.is_some(),
            "清单里没有当前平台的条目（插件会依次找 {:?}）：运行时 check() 会报 ReleaseNotFound",
            keys
        );

        // 地址必须是可下载的 http(s) 直链（`Url` 已保证是合法 URL，这里再钉住协议）
        let url = release.download_url(hit.unwrap()).unwrap();
        assert!(
            url.scheme() == "https" || url.scheme() == "http",
            "平台下载地址不是 http(s) 直链：{url}"
        );
    }

    #[test]
    fn empty_signature_only_allowed_for_current_version() {
        let release = feed();
        let RemoteReleaseInner::Static { platforms } = &release.data else {
            panic!("清单用的是 dynamic 格式（顶层 url/signature）：本项目走 platforms 静态格式");
        };

        let unsigned: Vec<&String> = platforms
            .iter()
            .filter(|(_, p)| p.signature.trim().is_empty())
            .map(|(target, _)| target)
            .collect();

        if unsigned.is_empty() {
            return;
        }

        // 空签名 = 占位清单：只能描述「当前已发布版本」。一旦把它写高，
        // 检查更新会提示更新、下载完却在签名校验处失败，用户拿到的是一个装不上的更新。
        assert_eq!(
            release.version.to_string(),
            app_version(),
            "平台 {unsigned:?} 的 signature 为空，此时清单版本必须等于当前应用版本（占位语义）"
        );
    }

    /// 反例：空 url 会让**整份**清单解析失败 —— 旧的 latest.json 里
    /// `linux-x86_64.url = ""` 就是这类，哪怕当前平台条目是好的
    #[test]
    fn empty_platform_url_invalidates_the_whole_manifest() {
        let bad = r#"{
          "version": "1.4.0",
          "platforms": {
            "linux-x86_64": { "signature": "", "url": "" },
            "windows-x86_64": { "signature": "x", "url": "https://example.com/a.msi" }
          }
        }"#;
        assert!(
            serde_json::from_str::<RemoteRelease>(bad).is_err(),
            "空 url 应当被拒：非当前平台的空占位也会让整份清单失效"
        );
    }

    /// 自动安装更新的打包前提：**NSIS 用户级安装 + 开启签名产物**。
    ///
    /// 为什么把这两条钉在测试里：
    /// - 更新包必须是 NSIS。MSI 是机器级安装，`msiexec` 需要管理员权限，每次自动更新都会弹
    ///   UAC（Tauri 官方对更新包的说明也是「别用 msi」）。NSIS 走用户级安装，更新全程静默。
    ///   把 msi 留在 targets 里，很容易被下一个人顺手写进清单。
    /// - `createUpdaterArtifacts` 必须为 true，否则构建不产 `.sig`，清单里没签名可填，
    ///   用户下载完会在验签处失败（本地完全无感，只有换台机器点更新才暴露）。
    #[test]
    fn windows_auto_update_needs_nsis_bundle_and_signature_artifacts() {
        let conf = app_conf();

        let targets: Vec<String> = conf
            .pointer("/bundle/targets")
            .and_then(|v| v.as_array())
            .expect("tauri.conf.json 缺少 bundle.targets")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        assert!(
            targets.iter().any(|t| t == "nsis"),
            "bundle.targets 里没有 nsis：自动更新需要 NSIS 安装包（当前 {targets:?}）"
        );
        assert!(
            !targets.iter().any(|t| t == "msi"),
            "bundle.targets 里仍有 msi：MSI 是机器级安装，自动更新每次都会弹 UAC 提权框"
        );

        assert_eq!(
            conf.pointer("/bundle/createUpdaterArtifacts").and_then(|v| v.as_bool()),
            Some(true),
            "bundle.createUpdaterArtifacts 没开：构建不会产出 .sig，更新包无法签名"
        );

        assert_eq!(
            conf.pointer("/bundle/windows/nsis/installMode").and_then(|v| v.as_str()),
            Some("currentUser"),
            "NSIS 安装模式不是 currentUser：perMachine 需要管理员权限，更新同样会弹 UAC"
        );
    }
}

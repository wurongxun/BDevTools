pub mod app_store;
pub mod kv_store;
pub mod tray;

/// 更新清单与 updater 端点配置的自检（仅测试构建，见文件头注释）
#[cfg(test)]
pub mod updater_feed;

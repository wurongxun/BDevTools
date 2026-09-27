/*! # 插件宿主机制

**这是「插件机制」本身，不是某个插件的业务**（插件业务在 `crate::plugins::<id>`）。

- `host` —— 清单解析、插件目录、`plugin://<id>/...` 资源协议、`__data__` 私有数据目录
- `wasm` —— WASM 沙箱：能力白名单、host function、按调用生命周期

信任模型是「安装即信任」：权限 = 诚实性校验 + 用户知情，**不是安全边界**。
详见文档 `docs/plugin-dev/capabilities.md`。
*/
pub mod host;
pub mod wasm;

import { call } from '@/api/rpc'

// 通用请求封装（历史兼容名，新代码请直接用 @/api/rpc 的 call）
export function tauri_request<T>(path:string,params:{}): Promise<T> {
  return call<T>(path,params);
}
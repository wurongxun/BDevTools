import { invoke } from "@tauri-apps/api/core";

//通用请求封装
export function tauri_request<T>(path:string,params:{}): Promise<T> {
  return invoke<T>(path,params);
}
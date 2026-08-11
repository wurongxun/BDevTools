import { createPinia } from 'pinia'
import systemStore from './modules/system'
import userStore from './modules/user'
import permissionStore from './modules/permission'
import errorInfoStore from './modules/errorInfo'
import { appConfigInfo } from './modules/appConfigInfo'
// import { useNoteStore } from './modules/noteTem'
import { fileTreeStore } from './modules/fileTree'
import { useWorkspaceStore } from './modules/workspace'
import { useSessionStore } from './modules/session'
import useLayoutStore from './modules/layout'
const pinia = createPinia()

export default function useStore() {
  return {
    system: systemStore(pinia),
    user: userStore(pinia),
    permission: permissionStore(pinia),
    errorStore: errorInfoStore(pinia),
    appConfig: appConfigInfo(pinia),
    fileTreeStore: fileTreeStore(pinia),
    useWorkspaceStore: useWorkspaceStore(pinia),
    useSessionStore: useSessionStore(pinia),
    useLayoutStore: useLayoutStore(pinia)
  }
}
//noteTem: useNoteStore(pinia),
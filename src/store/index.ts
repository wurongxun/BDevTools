import { createPinia } from 'pinia'
import systemStore from './modules/system'
import userStore from './modules/user'
import permissionStore from './modules/permission'
import errorInfoStore from './modules/errorInfo'
import { appConfigInfo } from './modules/appConfigInfo'
// import { useNoteStore } from './modules/noteTem'
import { routerNoteStore } from './modules/routerNote'
import { useDocumentManagerStore } from './modules/documents'
import { useConnectionStore } from './modules/connection'
import { useFtpStore } from './modules/ftp'
import { useWorkspaceStore } from './modules/workspace'
import { useSessionStore } from './modules/session'
const pinia = createPinia()

export default function useStore() {
  return {
    system: systemStore(pinia),
    user: userStore(pinia),
    permission: permissionStore(pinia),
    errorStore: errorInfoStore(pinia),
    appConfig: appConfigInfo(pinia),
    routerNoteStore: routerNoteStore(pinia),
    useDocumentManagerStore: useDocumentManagerStore(pinia),
    useConnectionStore: useConnectionStore(pinia),
    useFtpStore: useFtpStore(pinia),
    useWorkspaceStore: useWorkspaceStore(pinia),
    useSessionStore: useSessionStore(pinia)
  }
}
//noteTem: useNoteStore(pinia),
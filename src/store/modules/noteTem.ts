/**
 * 笔记数据存储
 * 包含多个笔记及每个笔记的临时编辑内容和原笔迹内容
 *
 */
import { defineStore } from 'pinia'

// 定义笔记数据接口
interface NoteData {
  id: string
  title: string
  content: string
  tempContent?: string
  createdAt: Date
  updatedAt: Date
}

export const useNoteStore = defineStore('note', {
  state: () => ({
    notes: new Map<string, NoteData>(),
    activeNoteId: null as string | null
  }),

  getters: {
    currentNote: (state) => {
      if (!state.activeNoteId) return null
      return state.notes.get(state.activeNoteId) || null
    },

    noteCount: (state) => {
      return state.notes.size
    }
  },

  actions: {
    setActiveNote(id: string) {
      this.activeNoteId = id
    },

    setNoteData(id: string, data: Partial<NoteData>) {
      const existingNote = this.notes.get(id)
      if (existingNote) {
        // 如果笔记存在，则更新笔记
        this.notes.set(id, { ...existingNote, ...data, updatedAt: new Date() })
      } else {
        // 如果笔记不存在，则创建新笔记
        this.notes.set(id, {
          id,
          title: data.title || '未命名笔记',
          content: data.content || '',
          createdAt: new Date(),
          updatedAt: new Date(),
          ...data
        } as NoteData)
      }
    },

    getNoteData(id: string) {
      return this.notes.get(id) || null
    },

    removeNote(id: string) {
      this.notes.delete(id)
      if (this.activeNoteId === id) {
        this.activeNoteId = null
      }
    },

    addNote(note: Partial<NoteData>) {
      const newNote: NoteData = {
        id: note.id || Date.now().toString(),
        title: note.title || '未命名笔记',
        content: note.content || '',
        createdAt: new Date(),
        updatedAt: new Date(),
        ...note
      }
      this.notes.set(newNote.id, newNote)
      return newNote.id
    },

    clearAllNotes() {
      this.notes.clear()
      this.activeNoteId = null
    }
  }
})

export default useNoteStore
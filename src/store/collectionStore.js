import { defineStore } from 'pinia'
import {
  FAVORITES_ID,
  addTrack,
  createPlaylist,
  deletePlaylist,
  emptyCollections,
  hasTrack,
  normalizeCollections,
  removeTrack,
  renamePlaylist,
  toggleFavorite,
  trackIdsFor,
} from '../utils/collectionModel.mjs'

let editRevision = 0
let savedRevision = 0
let pendingSnapshot = null
let saveLoop = null

export const useCollectionStore = defineStore('collectionStore', {
  state: () => ({
    ...emptyCollections(),
    hydrated: false,
    saveState: 'saved',
  }),
  getters: {
    favoriteCount: (state) => state.favoriteTrackIds.length,
    isFavorite: (state) => (trackId) => state.favoriteTrackIds.includes(trackId),
    playlistById: (state) => (playlistId) => state.playlists.find((item) => item.id === playlistId) || null,
    trackIdsByPlaylist: (state) => (playlistId) => trackIdsFor(state, playlistId),
  },
  actions: {
    applyCollections(value) {
      const normalized = normalizeCollections(value)
      this.version = normalized.version
      this.nextPlaylistId = normalized.nextPlaylistId
      this.favoriteTrackIds = normalized.favoriteTrackIds
      this.playlists = normalized.playlists
    },
    snapshot() {
      return normalizeCollections(this)
    },
    async hydrate() {
      const value = await windowApi.getCollections()
      this.applyCollections(value)
      this.hydrated = true
    },
    async persist() {
      pendingSnapshot = this.snapshot()
      editRevision += 1
      this.saveState = 'saving'

      if (!saveLoop) {
        saveLoop = (async () => {
          try {
            while (savedRevision < editRevision) {
              const revision = editRevision
              const snapshot = pendingSnapshot
              const saved = await windowApi.saveCollections(JSON.stringify(snapshot))
              savedRevision = revision
              if (revision === editRevision) this.applyCollections(saved)
            }
            this.saveState = 'saved'
          } catch (error) {
            this.saveState = 'failed'
            throw error
          } finally {
            saveLoop = null
          }
        })()
      }
      return saveLoop
    },
    async toggleFavorite(trackId) {
      this.applyCollections(toggleFavorite(this, trackId))
      await this.persist()
      return this.isFavorite(trackId)
    },
    async createPlaylist(name) {
      const result = createPlaylist(this, name)
      this.applyCollections(result.collections)
      await this.persist()
      return result.playlist
    },
    async renamePlaylist(playlistId, name) {
      this.applyCollections(renamePlaylist(this, playlistId, name))
      await this.persist()
    },
    async deletePlaylist(playlistId) {
      this.applyCollections(deletePlaylist(this, playlistId))
      await this.persist()
    },
    async addTrack(playlistId, trackId) {
      const alreadyAdded = hasTrack(this, playlistId, trackId)
      this.applyCollections(addTrack(this, playlistId, trackId))
      if (!alreadyAdded) await this.persist()
      return !alreadyAdded
    },
    async removeTrack(playlistId, trackId) {
      const existed = hasTrack(this, playlistId, trackId)
      this.applyCollections(removeTrack(this, playlistId, trackId))
      if (existed) await this.persist()
      return existed
    },
    isReservedPlaylist(playlistId) {
      return playlistId === FAVORITES_ID
    },
  },
})

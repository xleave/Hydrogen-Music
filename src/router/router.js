import { createRouter, createWebHashHistory } from 'vue-router'
import MyMusic from '../views/MyMusic.vue'
import LocalMusicDetail from '../components/LocalMusicDetail.vue'
import Settings from '../views/Settings.vue'
import { useLocalStore } from '../store/localStore'

const localStore = useLocalStore()

const routes = [
  {
    path: '/',
    redirect: '/mymusic',
  },
  {
    path: '/mymusic',
    name: 'mymusic',
    component: MyMusic,
    children: [
      {
        path: 'local/files',
        name: 'localFiles',
        component: LocalMusicDetail,
        beforeEnter: (to) => localStore.updateLocalMusicDetail(to.name, to.query),
      },
      {
        path: 'local/album/:id',
        name: 'localAlbum',
        component: LocalMusicDetail,
        beforeEnter: (to) => localStore.updateLocalMusicDetail(to.name, null, to.params.id),
      },
      {
        path: 'local/artist/:id',
        name: 'localArtist',
        component: LocalMusicDetail,
        beforeEnter: (to) => localStore.updateLocalMusicDetail(to.name, null, to.params.id),
      },
    ],
  },
  {
    path: '/settings',
    name: 'settings',
    component: Settings,
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/mymusic',
  },
]

export default createRouter({
  history: createWebHashHistory(),
  routes,
})

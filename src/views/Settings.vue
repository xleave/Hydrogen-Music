<script setup>
import { computed, onActivated, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRouter } from 'vue-router'
import { dialogOpen } from '../utils/dialog'
import {
  flushSettings,
  initSettings,
  resetSettingsPersistence,
  scheduleSettingsSave,
  settingsSaveState,
} from '../utils/initApp'
import { usePlayerStore } from '../store/playerStore'
import { insertCustomFontStyle } from '../utils/setFont'
import Selector from '../components/Selector.vue'
import SettingButton from '../components/settings/SettingButton.vue'
import SettingToggle from '../components/settings/SettingToggle.vue'

const router = useRouter()
const playerStore = usePlayerStore()

const lyricSize = ref(20)
const tlyricSize = ref(13)
const rlyricSize = ref(12)
const lyricInterlude = ref(13)
const globalShortcuts = ref(false)
const globalShortcutError = ref('')
const quitApp = ref('minimize')
const quitAppOptions = [
  { label: '最小化至托盘', value: 'minimize' },
  { label: '直接退出', value: 'quit' },
]
const localFolder = ref([])
const shortcutsList = ref([])
const selectedShortcut = ref(null)
const newShortcut = ref([])
const shortcutCharacter = ['=', '-', '~', '@', '#', '$', '[', ']', ';', "'", ',', '.', '/', '!']
const customFont = ref('')
const fontOptions = ref([{ label: '使用应用默认字体', value: '' }])
const fontLoading = ref(false)
const hydrating = ref(true)
const saveState = settingsSaveState

const saveStateLabel = computed(() => ({
  saved: '已自动保存',
  saving: '保存中…',
  failed: '保存失败',
}[saveState.value]))

const globalShortcutState = computed(() => {
  if (!globalShortcuts.value) return { label: '窗口内快捷键可用', failed: false }
  if (globalShortcutError.value) return { label: '当前会话不支持全局快捷键', failed: true }
  return { label: '全局快捷键已注册', failed: false }
})

function buildSettings() {
  return {
    music: {
      lyricSize: lyricSize.value,
      tlyricSize: tlyricSize.value,
      rlyricSize: rlyricSize.value,
      lyricInterlude: lyricInterlude.value,
    },
    local: {
      localFolder: localFolder.value,
    },
    shortcuts: shortcutsList.value,
    other: {
      globalShortcuts: globalShortcuts.value,
      quitApp: quitApp.value,
      customFont: customFont.value,
    },
  }
}

function scheduleSave() {
  if (hydrating.value) return
  scheduleSettingsSave(buildSettings())
}

async function applyShortcuts() {
  if (hydrating.value || selectedShortcut.value) return
  try {
    await windowApi.registerShortcuts(shortcutsList.value, globalShortcuts.value)
    globalShortcutError.value = ''
  } catch (error) {
    globalShortcutError.value = String(error)
    console.error('[shortcuts.register]', error)
  }
}

async function loadSystemFonts() {
  if (fontOptions.value.length > 1 || fontLoading.value) return
  fontLoading.value = true
  try {
    const fonts = await windowApi.listSystemFonts()
    const options = (fonts || []).map((family) => ({ label: family, value: family }))
    fontOptions.value = [{ label: '使用应用默认字体', value: '' }, ...options]
  } catch (error) {
    console.error('[fonts.list]', error)
  } finally {
    fontLoading.value = false
  }
}

function ensureCurrentFontOption() {
  if (!customFont.value) return
  if (fontOptions.value.some((item) => item.value === customFont.value)) return
  fontOptions.value.push({ label: customFont.value, value: customFont.value })
}

async function hydrateSettings() {
  hydrating.value = true
  resetSettingsPersistence()
  const [settings] = await Promise.all([
    windowApi.getSettings(),
    loadSystemFonts(),
  ])
  if (settings) {
    lyricSize.value = settings.music.lyricSize
    tlyricSize.value = settings.music.tlyricSize
    rlyricSize.value = settings.music.rlyricSize
    lyricInterlude.value = settings.music.lyricInterlude
    localFolder.value = [...(settings.local.localFolder || [])]
    shortcutsList.value = settings.shortcuts || []
    globalShortcuts.value = settings.other.globalShortcuts
    quitApp.value = settings.other.quitApp
    customFont.value = settings.other.customFont || ''
  }
  ensureCurrentFontOption()
  insertCustomFontStyle(customFont.value)
  hydrating.value = false
  await applyShortcuts()
}

onActivated(hydrateSettings)

watch(
  [lyricSize, tlyricSize, rlyricSize, lyricInterlude, localFolder, shortcutsList, globalShortcuts, quitApp],
  scheduleSave,
  { deep: true },
)

watch(customFont, (value) => {
  insertCustomFontStyle(value)
  scheduleSave()
})

watch(
  [shortcutsList, globalShortcuts],
  () => applyShortcuts(),
  { deep: true },
)

watch(selectedShortcut, (value, previous) => {
  if (!value && previous) applyShortcuts()
})

onBeforeRouteLeave(async () => {
  await flushSettings().catch((error) => console.error('[settings.flush]', error))
  await initSettings()
})

function routerChange() {
  router.back()
}

function selectFolder(type) {
  if (type !== 'local') return
  windowApi.openFile().then((path) => {
    if (path && !localFolder.value.includes(path)) localFolder.value.push(path)
  })
}

function deleteLocalFolder(index) {
  localFolder.value.splice(index, 1)
}

function formatShortcutName(name = '') {
  return name
    .replaceAll('+', ' + ')
    .replace('Up', '↑')
    .replace('Down', '↓')
    .replace('Right', '→')
    .replace('Left', '←')
    .replace('Space', '空格')
    .replace('Numpad', '')
    .replace('num', '')
    .replace('CommandOrControl', 'Ctrl')
    .replace('Control', 'Ctrl')
}

function changeShortcut(id, type) {
  selectedShortcut.value = { id, type }
  windowApi.unregisterShortcuts().catch((error) => console.error('[shortcuts.unregister]', error))
}

function updateShortcut() {
  const shortcut = []
  newShortcut.value.forEach((event) => {
    if (event.keyCode >= 65 && event.keyCode <= 90) shortcut.push(event.code.replace('Key', ''))
    else if (['Control', 'Shift', 'Alt'].includes(event.key)) shortcut.push(event.key)
    else if (event.keyCode >= 48 && event.keyCode <= 57) shortcut.push(event.code.replace('Digit', ''))
    else if (event.keyCode >= 96 && event.keyCode <= 105) shortcut.push(event.code.replace('Numpad', 'num'))
    else if (event.keyCode >= 112 && event.keyCode <= 123) shortcut.push(event.code)
    else if (['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown'].includes(event.key)) shortcut.push(event.code.replace('Arrow', ''))
    else if (shortcutCharacter.includes(event.key)) shortcut.push(event.key)
  })
  const sortTable = { Control: 1, Shift: 2, Alt: 3 }
  shortcut.sort((a, b) => (sortTable[a] || 99) - (sortTable[b] || 99))
  return shortcut.join('+')
}

function inputShortcut(event) {
  if (!selectedShortcut.value) return
  event.preventDefault()
  event.stopPropagation()
  if (newShortcut.value.find((key) => key.keyCode === event.keyCode)) return
  newShortcut.value.push(event)
  const isComplete = (event.keyCode >= 65 && event.keyCode <= 90)
    || (event.keyCode >= 48 && event.keyCode <= 57)
    || (event.keyCode >= 96 && event.keyCode <= 105)
    || (event.keyCode >= 112 && event.keyCode <= 123)
    || ['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown'].includes(event.key)
    || shortcutCharacter.includes(event.key)
  if (!isComplete) return

  const target = shortcutsList.value.find((shortcut) => shortcut.id === selectedShortcut.value.id)
  if (target) {
    if (selectedShortcut.value.type) target.globalShortcut = updateShortcut()
    else target.shortcut = updateShortcut()
  }
  newShortcut.value = []
  selectedShortcut.value = null
}

function setDefaultShortcuts() {
  shortcutsList.value = [
    { id: 'play', name: '播放/暂停', shortcut: 'CommandOrControl+P', globalShortcut: 'CommandOrControl+Alt+P' },
    { id: 'last', name: '上一首', shortcut: 'CommandOrControl+Left', globalShortcut: 'CommandOrControl+Alt+Left' },
    { id: 'next', name: '下一首', shortcut: 'CommandOrControl+Right', globalShortcut: 'CommandOrControl+Alt+Right' },
    { id: 'volumeUp', name: '增加音量', shortcut: 'CommandOrControl+Up', globalShortcut: 'CommandOrControl+Alt+Up' },
    { id: 'volumeDown', name: '减少音量', shortcut: 'CommandOrControl+Down', globalShortcut: 'CommandOrControl+Alt+Down' },
    { id: 'processForward', name: '快进(3s)', shortcut: 'CommandOrControl+]', globalShortcut: 'CommandOrControl+Alt+]' },
    { id: 'processBack', name: '后退(3s)', shortcut: 'CommandOrControl+[', globalShortcut: 'CommandOrControl+Alt+[' },
  ]
}

function setCoverBlur() {
  if (!playerStore.coverBlur) {
    dialogOpen('确定开启', '开启后此功能会消耗一定性能且可能造成卡顿，确定开启吗？', openCoverBlur)
  } else {
    openCoverBlur(true)
  }
}

function openCoverBlur(flag) {
  if (flag) playerStore.coverBlur = !playerStore.coverBlur
}

function setLyricBlur() {
  if (!playerStore.lyricBlur) {
    dialogOpen('确定开启', '开启后此功能会消耗一定性能且可能造成卡顿，确定开启吗？', openLyricBlur)
  } else {
    openLyricBlur(true)
  }
}

function openLyricBlur(flag) {
  if (flag) playerStore.lyricBlur = !playerStore.lyricBlur
}

function toGithub() {
  windowApi.toRegister('https://github.com/xleave/Hydrogen-Music')
}
</script>

<template>
  <div class="settings-page" @click="selectedShortcut = null">
    <div class="view-control">
      <svg @click="routerChange" class="router-last" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
        <path d="M716.608 1010.112L218.88 512.384 717.376 13.888l45.248 45.248-453.248 453.248 452.48 452.48z" />
      </svg>
      <span class="setting-title">设置</span>
      <span class="save-status" :class="`save-${saveState}`">{{ saveStateLabel }}</span>
    </div>

    <div class="settings-container">
      <h1 class="settings-title">设置</h1>
      <div class="settings">
        <div class="settings-item">
          <h2 class="item-title">音乐</h2>
          <div class="line"></div>
          <div class="item-options">
            <div class="option">
              <div class="option-name">开启背景封面模糊</div>
              <div class="option-operation">
                <SettingToggle :active="playerStore.coverBlur" :label="playerStore.coverBlur ? '已开启' : '已关闭'" @toggle="setCoverBlur" />
              </div>
            </div>
            <div class="option">
              <div class="option-name">开启歌词模糊</div>
              <div class="option-operation">
                <SettingToggle :active="playerStore.lyricBlur" :label="playerStore.lyricBlur ? '已开启' : '已关闭'" @toggle="setLyricBlur" />
              </div>
            </div>
            <div class="option"><div class="option-name">歌词字体大小</div><div class="option-operation"><input v-model="lyricSize" name="lyricSize"></div></div>
            <div class="option"><div class="option-name">歌词翻译字体大小</div><div class="option-operation"><input v-model="tlyricSize" name="tlyricSize"></div></div>
            <div class="option"><div class="option-name">罗马歌词字体大小</div><div class="option-operation"><input v-model="rlyricSize" name="rlyricSize"></div></div>
            <div class="option"><div class="option-name">歌词间奏等待时间(单位：秒)</div><div class="option-operation"><input v-model="lyricInterlude" name="lyricInterlude"></div></div>
          </div>
        </div>

        <div class="settings-item">
          <h2 class="item-title">本地</h2>
          <div class="line"></div>
          <div class="item-options">
            <div class="option local-folder-option">
              <div class="option-name">本地目录</div>
              <div class="local-folder">
                <div class="selected-local-folder-item">
                  <div class="selected-folder" :title="item" @contextmenu.prevent="deleteLocalFolder(index)" v-for="(item, index) in localFolder" :key="item">{{ item || '请添加' }}</div>
                  <div class="tip">可添加多个目录；右键移除目录。大型音乐库扫描需要一定时间。</div>
                </div>
                <SettingButton class="add-option" padding="5px 15px" :font-size="13" @click="selectFolder('local')">添加</SettingButton>
              </div>
            </div>
          </div>
        </div>

        <div class="settings-item">
          <h2 class="item-title">快捷键</h2>
          <div class="line"></div>
          <div class="item-options" tabindex="0" @keydown="inputShortcut">
            <div class="option shortcut-toggle-option">
              <div class="shortcut-option-copy">
                <div class="option-name">开启全局快捷键</div>
                <div class="shortcut-status" :class="{ 'shortcut-status-failed': globalShortcutState.failed }">{{ globalShortcutState.label }}</div>
              </div>
              <div class="option-operation">
                <SettingToggle
                  :active="globalShortcuts && !globalShortcutState.failed"
                  :label="globalShortcuts ? (globalShortcutState.failed ? '不可用' : '已开启') : '已关闭'"
                  @toggle="globalShortcuts = !globalShortcuts"
                />
              </div>
            </div>
            <div class="shortcuts-title">
              <div class="title-function">功能说明</div>
              <div class="title-shortcuts">快捷键</div>
              <div class="title-globalShortcuts" :class="{ 'forbid-shortcuts': !globalShortcuts }">全局快捷键</div>
            </div>
            <div class="shortcuts" v-for="item in shortcutsList" :key="item.id">
              <div class="shortcut-name">{{ item.name }}</div>
              <div class="shortcut" :class="{ 'shortcut-selected': selectedShortcut?.id === item.id && !selectedShortcut?.type }" @click.stop="changeShortcut(item.id, false)">{{ formatShortcutName(item.shortcut) }}</div>
              <div class="globalShortcut" :class="{ 'shortcut-selected': selectedShortcut?.id === item.id && selectedShortcut?.type, 'forbid-shortcuts': !globalShortcuts }" @click.stop="changeShortcut(item.id, true)">{{ formatShortcutName(item.globalShortcut) }}</div>
            </div>
            <SettingButton class="default-shortcuts" :width="132" @click="setDefaultShortcuts">恢复默认快捷键</SettingButton>
          </div>
        </div>

        <div class="settings-item">
          <h2 class="item-title">其他</h2>
          <div class="line"></div>
          <div class="item-options">
            <div class="option font-option">
              <div class="option-name">自定义字体</div>
              <div class="font-control">
                <Selector
                  v-model="customFont"
                  :options="fontOptions"
                  :max-items="8"
                  searchable
                  :search-placeholder="fontLoading ? '正在读取系统字体…' : '搜索系统字体'"
                />
              </div>
            </div>
            <div class="option">
              <div class="option-name">退出应用时</div>
              <div class="option-operation"><Selector v-model="quitApp" :options="quitAppOptions" /></div>
            </div>
          </div>
        </div>
      </div>

      <div class="app-version">
        <div class="app-icon"><img src="../assets/icon/icon.ico" alt=""></div>
        <div class="version">V0.8.0</div>
        <div class="app-author" @click="toGithub">Made by xleave</div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-page {
  width: 100%;
  height: 100%;

  .view-control {
    margin-top: 14Px;
    margin-bottom: 15Px;
    margin-left: -8Px;
    height: 32Px;
    display: flex;
    align-items: center;

    svg {
      padding: 8Px;
      width: 32Px;
      height: 32Px;
      transition: .2s;
      &:hover { cursor: pointer; opacity: .7; }
      &:active { transform: scale(.9); }
    }

    .router-last { margin-right: 10Px; }
    .setting-title { font: 17Px SourceHanSansCN-Bold; color: black; }
    .save-status {
      margin-left: 20Px;
      padding-left: 14Px;
      border-left: .5Px solid rgba(0, 0, 0, .18);
      font: 11Px SourceHanSansCN-Bold;
      color: rgba(0, 0, 0, .45);
      transition: color .2s;
      &.save-failed { color: rgba(150, 30, 30, .8); }
      &.save-saving { color: rgba(0, 0, 0, .62); }
    }
  }

  .settings-container {
    margin: 0 auto;
    padding-bottom: 140px;
    width: 80%;
    height: calc(100% - 61px);
    overflow: auto;
    &::-webkit-scrollbar { display: none; }

    .settings-title {
      font-family: SourceHanSansCN-Bold;
      color: black;
      text-align: left;
    }

    .settings-item {
      margin-top: 45px;
      width: 100%;

      .item-title {
        margin: 0;
        font: 20Px SourceHanSansCN-Bold;
        color: black;
        text-align: left;
      }

      .line {
        margin-top: 8px;
        margin-bottom: 25px;
        width: 100%;
        height: .5px;
        background-color: rgba(0, 0, 0, .2);
      }

      .item-options {
        outline: none;

        .option {
          margin-bottom: 32px;
          display: flex;
          align-items: center;
          justify-content: space-between;

          .option-name {
            font-family: SourceHanSansCN-Bold;
            font-size: 16px;
            color: black;
            text-align: left;
          }

          input,
          :deep(.selector) {
            margin-right: 1px;
            width: 200px;
            height: 34px;
            padding: 5px 1px;
            box-sizing: border-box;
            background-color: rgba(255, 255, 255, .35);
            color: black;
            border: none;
            outline: none;
            appearance: none;
            font: 13px SourceHanSansCN-Bold;
            text-align: center;
            transition: .2s;
            &:hover { cursor: pointer; opacity: .8; box-shadow: inset 0 0 0 1px black; }
          }

          .local-folder {
            display: flex;
            align-items: center;
            .selected-local-folder-item {
              display: flex;
              flex-direction: column;
              .selected-folder {
                margin-bottom: 10px;
                padding: 0 8px;
                box-sizing: border-box;
                width: 50vw;
                height: 30px;
                background-color: rgba(255, 255, 255, .35);
                font: 13px SourceHanSansCN-Bold;
                color: black;
                line-height: 30px;
                overflow: hidden;
                text-overflow: ellipsis;
                white-space: nowrap;
              }
              .tip { font: 10px SourceHanSansCN-Bold; color: black; text-align: left; }
            }
            .add-option { margin-left: 15px; }
          }

          .font-control {
            width: 200px;
            display: flex;
            flex-direction: column;
            gap: 0;
            :deep(.selector) { width: 200px; }
          }
        }

        .shortcut-toggle-option { align-items: flex-start; }
        .shortcut-option-copy {
          display: flex;
          flex-direction: column;
          align-items: flex-start;
          gap: 4px;
        }
        .shortcut-status {
          font: 10px SourceHanSansCN-Bold;
          color: rgba(0, 0, 0, .48);
        }
        .shortcut-status-failed { color: rgba(150, 30, 30, .82); }
        .local-folder-option { align-items: flex-start; }
        .font-option { align-items: flex-start; }
        .forbid-shortcuts { opacity: .5; pointer-events: none; }

        .shortcuts-title,
        .shortcuts {
          font: 14px SourceHanSansCN-Bold;
          color: black;
          display: flex;
          align-items: center;
          text-align: left;
          > div { margin-right: 15px; padding: 0 6px; }
          .title-function,
          .shortcut-name { min-width: 130px; }
          .title-shortcuts,
          .title-globalShortcuts,
          .shortcut,
          .globalShortcut { min-width: 200px; }
        }

        .shortcuts {
          > div { margin-top: 15px; padding: 6px; background-color: rgba(255, 255, 255, .35); }
          .shortcut-name { background-color: transparent; }
          .shortcut,
          .globalShortcut { &:hover { cursor: pointer; } }
          .shortcut-selected { box-shadow: inset 0 0 0 1px black; }
        }

        .default-shortcuts { margin-top: 15px; }
      }
    }

    .app-version {
      display: flex;
      flex-direction: column;
      align-items: center;
      .app-icon { margin-bottom: 10px; width: 65px; height: 65px; img { width: 100%; height: 100%; } }
      .version { font: 14px Geometos; color: black; }
      .app-author {
        margin-top: 10px;
        font: 14px Bender-Bold;
        color: black;
        &:hover { cursor: pointer; text-decoration: underline; }
      }
    }
  }
}
</style>

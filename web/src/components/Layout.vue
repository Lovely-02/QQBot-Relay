<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useThemeStore } from '../stores/theme'
import SvgIcon from './SvgIcon.vue'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const themeStore = useThemeStore()

const drawerOpen = ref(false)
const isMobile = ref(false)
const collapsed = ref(false)

const navItems = [
  { label: '仪表盘', key: 'Dashboard', icon: 'home' },
  { label: 'AppID', key: 'AppIds', icon: 'key' },
  { label: '二次转发', key: 'Webhook', icon: 'webhook' },
  { label: '数据库', key: 'Database', icon: 'server' },
  { label: '设置', key: 'Settings', icon: 'settings' }
]

const currentRouteName = computed(() => route.name)
const pageTitle = computed(() => {
  const item = navItems.find(n => n.key === route.name)
  return item ? item.label : ''
})

function navigate(name) {
  router.push({ name })
  drawerOpen.value = false
}

async function handleLogout() {
  await authStore.logout()
  router.push('/login')
}

function checkMobile() {
  isMobile.value = window.innerWidth < 768
}

onMounted(() => {
  checkMobile()
  window.addEventListener('resize', checkMobile)
})
onUnmounted(() => {
  window.removeEventListener('resize', checkMobile)
})
</script>

<template>
  <div class="layout-root">
    <div v-if="drawerOpen" class="mobile-overlay" @click="drawerOpen = false" />

    <aside :class="['sidebar', { open: drawerOpen, collapsed: collapsed && !isMobile }]">
      <div class="sidebar-logo">
        <SvgIcon name="webhook" :size="28" color="var(--accent)" />
        <span v-if="!collapsed || isMobile" class="logo-text">Bridge</span>
      </div>

      <nav class="sidebar-nav">
        <a
          v-for="item in navItems" :key="item.key"
          :class="['nav-item', { active: currentRouteName === item.key }]"
          @click="navigate(item.key)"
        >
          <SvgIcon :name="item.icon" :size="18" />
          <span v-if="!collapsed || isMobile">{{ item.label }}</span>
        </a>
      </nav>

      <div v-if="!isMobile" class="sidebar-toggle" @click="collapsed = !collapsed">
        <SvgIcon :name="collapsed ? 'chevron-forward' : 'chevron-back'" :size="16" />
      </div>
    </aside>

    <div class="main-area">
      <header class="topbar">
        <div class="topbar-left">
          <button v-if="isMobile" class="hamburger" @click="drawerOpen = !drawerOpen">
            <SvgIcon name="menu" :size="22" />
          </button>
          <span class="page-title">{{ pageTitle }}</span>
        </div>
        <div class="topbar-right">
          <n-popover trigger="click" placement="bottom-end">
            <template #trigger>
              <n-button quaternary circle size="small" title="主题">
                <template #icon><SvgIcon name="color-palette" :size="18" /></template>
              </n-button>
            </template>
            <div class="theme-picker">
              <div
                v-for="(t, key) in themeStore.THEMES" :key="key"
                :class="['theme-opt', { active: themeStore.themeName === key }]"
                @click="themeStore.setTheme(key)"
              >
                <span class="theme-dot" :style="{ background: t.accent }" />
                {{ t.name }}
              </div>
            </div>
          </n-popover>

          <n-button quaternary circle size="small" @click="handleLogout" title="退出">
            <template #icon><SvgIcon name="log-out" :size="18" /></template>
          </n-button>
        </div>
      </header>

      <main class="content">
        <router-view v-slot="{ Component }">
          <transition name="page" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<style scoped>
.layout-root { display: flex; height: 100vh; overflow: hidden; background: var(--bg); }
.sidebar { width: 220px; flex-shrink: 0; background: var(--bg2); display: flex; flex-direction: column; border-right: 1px solid var(--border); transition: width .2s, transform .25s; z-index: 100; }
.sidebar.collapsed { width: 64px; }
.sidebar.collapsed .logo-text, .sidebar.collapsed .nav-item span { display: none; }
.sidebar.collapsed .nav-item { justify-content: center; padding: 12px 0; }
.sidebar-logo { display: flex; align-items: center; gap: 10px; padding: 16px; border-bottom: 1px solid var(--border); }
.logo-text { color: var(--text); font-weight: 600; font-size: 16px; white-space: nowrap; }
.sidebar-nav { flex: 1; overflow-y: auto; padding: 8px 0; }
.nav-item { display: flex; align-items: center; gap: 10px; padding: 10px 16px; margin: 2px 8px; border-radius: 8px; color: var(--text2); cursor: pointer; transition: all .15s; text-decoration: none; font-size: 14px; }
.nav-item:hover { background: var(--border); color: var(--text); }
.nav-item.active { background: var(--accent); color: #fff; }
.sidebar-toggle { padding: 12px; text-align: center; border-top: 1px solid var(--border); cursor: pointer; color: var(--text3); }
.sidebar-toggle:hover { color: var(--text); }
.main-area { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 0; }
.topbar { height: 52px; flex-shrink: 0; padding: 0 16px; display: flex; align-items: center; justify-content: space-between; background: var(--bg2); border-bottom: 1px solid var(--border); }
.topbar-left { display: flex; align-items: center; gap: 8px; }
.topbar-right { display: flex; align-items: center; gap: 4px; }
.page-title { color: var(--text); font-weight: 600; font-size: 15px; }
.hamburger { background: none; border: none; color: var(--text2); cursor: pointer; padding: 4px; display: flex; align-items: center; }
.content { flex: 1; overflow-y: auto; padding: 20px; }
.page-enter-active, .page-leave-active { transition: opacity .15s; }
.page-enter-from, .page-leave-to { opacity: 0; }
.mobile-overlay { position: fixed; top: 0; right: 0; bottom: 0; left: 0; background: rgba(0,0,0,0.5); z-index: 99; }
.theme-picker { display: flex; flex-direction: column; gap: 4px; min-width: 120px; }
.theme-opt { display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-radius: 6px; cursor: pointer; color: var(--text2); font-size: 13px; }
.theme-opt:hover { background: var(--border); color: var(--text); }
.theme-opt.active { color: var(--accent); font-weight: 600; }
.theme-dot { width: 12px; height: 12px; border-radius: 50%; flex-shrink: 0; }
@media (max-width: 767px) {
  .sidebar { position: fixed; left: 0; top: 0; bottom: 0; width: 260px; transform: translate(-100%); }
  .sidebar.open { transform: translate(0); }
  .content { padding: 12px; }
}
</style>

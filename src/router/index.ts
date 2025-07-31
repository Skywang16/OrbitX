import { createRouter, createWebHistory } from 'vue-router'

// 路由组件懒加载
const TerminalView = () => import('@/views/Terminal/TerminalView.vue')
const SettingsView = () => import('@/views/Settings/SettingsView.vue')

const routes = [
  {
    path: '/',
    name: 'Terminal',
    component: TerminalView,
    meta: {
      title: '终端',
    },
  },
  {
    path: '/settings',
    name: 'Settings',
    component: SettingsView,
    meta: {
      title: '设置',
    },
  },

  // 重定向未匹配的路由到主页
  {
    path: '/:pathMatch(.*)*',
    redirect: '/',
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

// 路由守卫 - 设置页面标题
router.beforeEach((to, _from, next) => {
  if (to.meta?.title) {
    document.title = `${to.meta.title} - TermX`
  }
  next()
})

export default router

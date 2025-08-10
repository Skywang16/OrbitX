import { createRouter, createWebHistory } from 'vue-router'

// 路由组件懒加载
const TerminalView = () => import('@/views/Terminal/TerminalView.vue')

const routes = [
  {
    path: '/',
    name: 'Terminal',
    component: TerminalView,
    meta: {
      title: 'OrbitX',
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
    document.title = to.meta.title
  }
  next()
})

export default router

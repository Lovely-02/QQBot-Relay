import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const routes = [
	{
		path: '/login',
		name: 'Login',
		component: () => import('../views/Login.vue'),
		meta: { guest: true }
	},
	{
		path: '/',
		component: () => import('../components/Layout.vue'),
		meta: { auth: true },
		children: [
			{ path: '', name: 'Dashboard', component: () => import('../views/Dashboard.vue') },
			{ path: 'appids', name: 'AppIds', component: () => import('../views/AppIds.vue') },
			{ path: 'webhook', name: 'Webhook', component: () => import('../views/Webhook.vue') },
			{ path: 'settings', name: 'Settings', component: () => import('../views/Settings.vue') },
			{ path: 'database', name: 'Database', component: () => import('../views/Database.vue') }
		]
	}
]

const router = createRouter({
	history: createWebHistory('/web'),
	routes
})

router.beforeEach(async (to, from, next) => {
	const auth = useAuthStore()
	if (to.meta.auth && !auth.isLoggedIn && !(await auth.checkAuth())) {
		return next({ name: 'Login', query: { redirect: to.fullPath } })
	}
	if (to.meta.guest && auth.isLoggedIn && (await auth.checkAuth())) {
		return next({ name: 'Dashboard' })
	}
	next()
})

export default router

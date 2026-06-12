import { createApp } from 'vue'
import { createPinia } from 'pinia'
import naive from 'naive-ui'
import App from './App.vue'
import router from './router'
import { setUnauthorizedHandler } from './api'
import { useAuthStore } from './stores/auth'
import './style.css'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(router)
app.use(naive)

setUnauthorizedHandler(() => {
	const auth = useAuthStore(pinia)
	auth.invalidate()

	const current = router.currentRoute.value
	if (current.name !== 'Login') {
		router.replace({ name: 'Login', query: { redirect: current.fullPath } })
	}
})

app.mount('#app')

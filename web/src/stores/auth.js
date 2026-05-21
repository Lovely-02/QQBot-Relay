import { defineStore } from 'pinia'
import { ref } from 'vue'
import { http } from '../api'

export const useAuthStore = defineStore('auth', () => {
	const isLoggedIn = ref(!!localStorage.getItem('wb_logged'))

	async function login(password) {
		try {
			await http.post('/api/admin/login', { password })
			isLoggedIn.value = true
			localStorage.setItem('wb_logged', '1')
		} catch (err) {
			throw new Error(err.response?.data?.error || '登录失败')
		}
	}

	async function checkAuth() {
		try {
			await http.get('/api/admin/verify')
			isLoggedIn.value = true
			localStorage.setItem('wb_logged', '1')
			return true
		} catch {
			isLoggedIn.value = false
			localStorage.removeItem('wb_logged')
			return false
		}
	}

	async function logout() {
		try {
			await http.post('/api/admin/logout')
		} catch {}
		isLoggedIn.value = false
		localStorage.removeItem('wb_logged')
	}

	return { isLoggedIn, login, checkAuth, logout }
})

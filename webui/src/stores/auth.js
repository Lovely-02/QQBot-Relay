import { defineStore } from 'pinia'
import { ref } from 'vue'
import { http } from '../api'

export const useAuthStore = defineStore('auth', () => {
	const isLoggedIn = ref(!!localStorage.getItem('wb_logged'))
	const authChecked = ref(false)

	function setAuthenticated(value) {
		isLoggedIn.value = value
		authChecked.value = true
		if (value) {
			localStorage.setItem('wb_logged', '1')
		} else {
			localStorage.removeItem('wb_logged')
		}
	}

	async function login(password) {
		try {
			await http.post('/api/admin/login', { password })
			setAuthenticated(true)
		} catch (err) {
			throw new Error(err.response?.data?.error || '登录失败')
		}
	}

	async function checkAuth() {
		try {
			await http.get('/api/admin/verify')
			setAuthenticated(true)
			return true
		} catch {
			setAuthenticated(false)
			return false
		}
	}

	async function ensureAuth() {
		if (authChecked.value) return isLoggedIn.value
		return checkAuth()
	}

	function invalidate() {
		setAuthenticated(false)
	}

	async function logout() {
		try {
			await http.post('/api/admin/logout')
		} catch {}
		setAuthenticated(false)
	}

	return { isLoggedIn, authChecked, login, checkAuth, ensureAuth, invalidate, logout }
})

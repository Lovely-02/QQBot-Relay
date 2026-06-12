import axios from 'axios'

const http = axios.create({
	baseURL: '/',
	withCredentials: true
})

let unauthorizedHandler = null

http.interceptors.response.use(
	(response) => response,
	(error) => {
		const status = error.response?.status
		const url = error.config?.url || ''
		const isAuthProbe = url.endsWith('/api/admin/verify')
		const isLogin = url.endsWith('/api/admin/login')

		if (status === 401 && !isAuthProbe && !isLogin) {
			unauthorizedHandler?.()
		}

		return Promise.reject(error)
	}
)

function setUnauthorizedHandler(handler) {
	unauthorizedHandler = handler
}

export { http, setUnauthorizedHandler }

import axios from 'axios'

const http = axios.create({
	baseURL: '/',
	withCredentials: true
})

export { http }

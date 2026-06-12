<script setup>
	import { ref, reactive } from 'vue'
	import { useRouter, useRoute } from 'vue-router'
	import { useMessage } from 'naive-ui'
	import { useAuthStore } from '../stores/auth'

	const router = useRouter()
	const route = useRoute()
	const message = useMessage()
	const authStore = useAuthStore()
	const loading = ref(false)
	const form = reactive({ password: '' })

	async function handleLogin() {
		if (!form.password) {
			message.warning('请输入密码')
			return
		}
		loading.value = true
		try {
			await authStore.login(form.password)
			message.success('登录成功')
			router.push(route.query.redirect || '/')
		} catch (err) {
			message.error(err.message || '登录失败')
		} finally {
			loading.value = false
		}
	}
</script>

<template>
	<div class="login-page">
		<div class="login-box">
			<div class="login-logo">
				<svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="1.2">
					<path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
				</svg>
				<h1>QQBot Relay</h1>
				<p>管理面板登录</p>
			</div>
			<n-card class="login-card" :bordered="false">
				<n-form @submit.prevent="handleLogin">
					<n-form-item label="密码">
						<n-input
							v-model:value="form.password"
							type="password"
							show-password-on="click"
							placeholder="管理员密码"
							size="large"
							@keyup.enter="handleLogin"
						/>
					</n-form-item>
					<n-button type="primary" block size="large" :loading="loading" @click="handleLogin" class="login-btn">
						登 录
					</n-button>
				</n-form>
			</n-card>
		</div>
	</div>
</template>

<style scoped>
	.login-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg);
	}
	.login-box {
		width: 100%;
		max-width: 380px;
		padding: 0 16px;
	}
	.login-logo {
		text-align: center;
		margin-bottom: 32px;
	}
	.login-logo svg {
		margin: 0 auto 16px;
		display: block;
	}
	.login-logo h1 {
		color: var(--text);
		font-size: 22px;
		font-weight: 700;
		margin: 0 0 4px;
	}
	.login-logo p {
		color: var(--text2);
		font-size: 14px;
		margin: 0;
	}
	.login-card {
		background: var(--bg2) !important;
		border: 1px solid var(--border) !important;
	}
	.login-btn {
		background: linear-gradient(135deg, var(--accent), var(--accent-light)) !important;
		border: none !important;
	}
</style>

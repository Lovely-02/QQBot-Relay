<script setup>
	import { ref, reactive, onMounted } from 'vue'
	import { useMessage } from 'naive-ui'
	import { http } from '../api'
	import SvgIcon from '../components/SvgIcon.vue'

	const message = useMessage()
	const loading = ref(false)
	const form = reactive({
		log_level: 'INFO',
		deduplication_ttl: 20,
		admin_enabled: true,
		admin_password: '',
		trust_proxy_headers: false,
		max_public_messages: 1000,
		max_token_messages: 500,
		message_ttl: 300,
		clean_interval: 120,
		webhook_timeout: 5,
		raw_enabled: false,
		raw_path: 'logs'
	})

	const logLevelOptions = [
		{ label: 'DEBUG', value: 'DEBUG' },
		{ label: 'INFO', value: 'INFO' },
		{ label: 'WARNING', value: 'WARNING' },
		{ label: 'ERROR', value: 'ERROR' }
	]

	async function fetchSettings() {
		try {
			const { data } = await http.get('/api/admin/settings')
			form.log_level = (data.log_level || 'INFO').toUpperCase()
			form.deduplication_ttl = data.deduplication_ttl ?? 20
			form.admin_enabled = data.admin?.enabled ?? true
			form.trust_proxy_headers = data.admin?.trust_proxy_headers ?? false
			form.max_public_messages = data.cache?.max_public_messages ?? 1000
			form.max_token_messages = data.cache?.max_token_messages ?? 500
			form.message_ttl = data.cache?.message_ttl ?? 300
			form.clean_interval = data.cache?.clean_interval ?? 120
			form.webhook_timeout = data.webhook_forward?.timeout ?? 5
			form.raw_enabled = data.raw_content?.enabled ?? false
			form.raw_path = data.raw_content?.path ?? 'logs'
		} catch {}
	}

	async function handleSave() {
		loading.value = true
		try {
			await http.post('/api/admin/settings/update', {
				log_level: form.log_level.toLowerCase(),
				deduplication_ttl: form.deduplication_ttl,
				admin: {
					enabled: form.admin_enabled,
					password: form.admin_password,
					trust_proxy_headers: form.trust_proxy_headers
				},
				cache: {
					max_public_messages: form.max_public_messages,
					max_token_messages: form.max_token_messages,
					message_ttl: form.message_ttl,
					clean_interval: form.clean_interval
				},
				webhook_forward: { timeout: form.webhook_timeout },
				raw_content: { enabled: form.raw_enabled, path: form.raw_path }
			})
			form.admin_password = ''
			message.success('设置已保存')
		} catch (err) {
			message.error(err.response?.data?.error || '保存失败')
		} finally {
			loading.value = false
		}
	}

	onMounted(fetchSettings)
</script>

<template>
	<div>
		<h3 class="page-heading">系统设置</h3>
		<n-card :bordered="false" class="panel-card">
			<n-form label-placement="left" label-width="140" :model="form">
				<n-form-item label="日志级别">
					<n-select v-model:value="form.log_level" :options="logLevelOptions" style="width: 200px" />
				</n-form-item>
				<n-form-item label="去重有效期 (秒)">
					<n-input-number v-model:value="form.deduplication_ttl" :min="0" :max="3600" style="width: 200px" />
				</n-form-item>
				<n-form-item label="管理面板">
					<n-switch v-model:value="form.admin_enabled" />
				</n-form-item>
				<n-form-item label="新管理员密码">
					<n-input
						v-model:value="form.admin_password"
						type="password"
						autocomplete="new-password"
						show-password-on="click"
						placeholder="留空表示不修改"
						style="width: 260px"
					/>
				</n-form-item>
				<n-form-item label="信任反向代理 IP">
					<n-switch v-model:value="form.trust_proxy_headers" />
				</n-form-item>
				<n-form-item label="公共缓存上限">
					<n-input-number v-model:value="form.max_public_messages" :min="0" :max="100000" style="width: 200px" />
				</n-form-item>
				<n-form-item label="Token 缓存上限">
					<n-input-number v-model:value="form.max_token_messages" :min="0" :max="100000" style="width: 200px" />
				</n-form-item>
				<n-form-item label="消息缓存时间 (秒)">
					<n-input-number v-model:value="form.message_ttl" :min="0" :max="86400" style="width: 200px" />
				</n-form-item>
				<n-form-item label="缓存清理周期 (秒)">
					<n-input-number v-model:value="form.clean_interval" :min="1" :max="86400" style="width: 200px" />
				</n-form-item>
				<n-form-item label="转发超时 (秒)">
					<n-input-number v-model:value="form.webhook_timeout" :min="1" :max="300" style="width: 200px" />
				</n-form-item>
				<n-form-item label="原始消息记录">
					<n-space align="center">
						<n-switch v-model:value="form.raw_enabled" />
						<n-input v-model:value="form.raw_path" placeholder="日志路径" style="width: 200px" />
					</n-space>
				</n-form-item>
				<n-form-item>
					<n-button type="primary" @click="handleSave" :loading="loading">
						<template #icon><SvgIcon name="save" :size="16" /></template>
						保存设置
					</n-button>
				</n-form-item>
			</n-form>
		</n-card>
	</div>
</template>

<style scoped>
	.page-heading {
		font-size: 16px;
		font-weight: 600;
		color: var(--text);
		margin-bottom: 16px;
	}
	.panel-card {
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 10px;
	}
</style>

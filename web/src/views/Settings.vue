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
				raw_content: { enabled: form.raw_enabled, path: form.raw_path }
			})
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

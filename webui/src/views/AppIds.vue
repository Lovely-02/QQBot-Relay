<script setup>
	import { ref, onMounted } from 'vue'
	import { useMessage } from 'naive-ui'
	import { http } from '../api'
	import SvgIcon from '../components/SvgIcon.vue'

	const message = useMessage()
	const appids = ref([])
	const showCreate = ref(false)
	const form = ref({ appid: '', secret: '', description: '' })

	function getCallbackUrl(item) {
		return `${location.origin}/webhook?secret=${encodeURIComponent(item.secret)}`
	}
	function getWsUrl(item) {
		const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
		return `${proto}//${location.host}/ws/${encodeURIComponent(item.secret)}`
	}
	function copy(text, label) {
		navigator.clipboard.writeText(text).then(
			() => message.success(`已复制${label}`),
			() => message.error('复制失败')
		)
	}
	function maskSecret(s) {
		if (!s) return '***'
		return s.length > 4 ? s.slice(0, 4) + '***' : '***'
	}

	async function fetchAppIds() {
		try {
			const { data } = await http.get('/api/admin/appids')
			appids.value = data
		} catch {}
	}

	async function handleCreate() {
		if (!form.value.appid || !form.value.secret || form.value.secret.length < 10) {
			message.warning('请输入有效的 AppID 和 Secret (>=10位)')
			return
		}
		try {
			await http.post('/api/admin/appids/create', form.value)
			message.success('创建成功')
			form.value = { appid: '', secret: '', description: '' }
			showCreate.value = false
			await fetchAppIds()
		} catch (err) {
			message.error(err.response?.data?.error || '创建失败')
		}
	}

	async function handleDelete(appid) {
		try {
			await http.delete(`/api/admin/appids/${appid}`)
			message.success('已删除')
			await fetchAppIds()
		} catch (err) {
			message.error(err.response?.data?.error || '删除失败')
		}
	}

	onMounted(fetchAppIds)
</script>

<template>
	<div>
		<div class="section-header">
			<h3>AppID 管理</h3>
			<n-button type="primary" size="small" @click="showCreate = !showCreate">
				<template #icon>
					<SvgIcon :name="showCreate ? 'chevron-back' : 'plus'" :size="14" />
				</template>
				{{ showCreate ? '取消' : '新增' }}
			</n-button>
		</div>

		<n-card v-if="showCreate" :bordered="false" class="create-card" size="small">
			<div class="create-form">
				<n-input v-model:value="form.appid" placeholder="AppID" size="small" />
				<n-input v-model:value="form.secret" placeholder="Secret (>=10位)" size="small" />
				<n-input v-model:value="form.description" placeholder="描述 (可选)" size="small" />
				<n-button type="primary" size="small" @click="handleCreate">创建</n-button>
			</div>
		</n-card>

		<div class="appid-list">
			<n-card v-for="item in appids" :key="item.appid" :bordered="false" class="appid-card">
				<div class="appid-header">
					<div class="appid-info">
						<code class="appid-id">{{ item.appid }}</code>
						<n-tag size="tiny" :bordered="false" type="info">{{ maskSecret(item.secret) }}</n-tag>
						<span v-if="item.description" class="appid-desc">{{ item.description }}</span>
					</div>
					<n-popconfirm @positive-click="handleDelete(item.appid)" positive-text="删除" negative-text="取消">
						<template #trigger>
							<n-button quaternary circle size="tiny" type="error" title="删除">
								<template #icon><SvgIcon name="trash" :size="14" /></template>
							</n-button>
						</template>
						确定要删除 AppID: {{ item.appid }}？
					</n-popconfirm>
				</div>

				<div class="appid-links">
					<div class="link-row" @click="copy(getCallbackUrl(item), '回调地址')">
						<SvgIcon name="globe" :size="14" />
						<span class="link-label">开放平台回调地址</span>
						<code class="link-url">{{ getCallbackUrl(item) }}</code>
						<SvgIcon name="copy" :size="13" class="copy-icon" />
					</div>
					<div class="link-row" @click="copy(getWsUrl(item), 'WebSocket 地址')">
						<SvgIcon name="zap" :size="14" />
						<span class="link-label">WebSocket 连接</span>
						<code class="link-url">{{ getWsUrl(item) }}</code>
						<SvgIcon name="copy" :size="13" class="copy-icon" />
					</div>
				</div>
			</n-card>

			<n-empty v-if="!appids.length" description="暂无 AppID" />
		</div>
	</div>
</template>

<style scoped>
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}
	.section-header h3 {
		font-size: 16px;
		font-weight: 600;
		color: var(--text);
		margin: 0;
	}
	.create-card {
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 10px;
		margin-bottom: 12px;
	}
	.create-form {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}
	.appid-list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.appid-card {
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 10px;
		transition: box-shadow 0.2s;
	}
	.appid-card:hover {
		box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
	}
	.appid-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 10px;
	}
	.appid-info {
		display: flex;
		align-items: center;
		gap: 8px;
		flex-wrap: wrap;
	}
	.appid-id {
		font-size: 15px;
		font-weight: 600;
		color: var(--text);
		background: var(--bg3);
		padding: 2px 8px;
		border-radius: 6px;
	}
	.appid-desc {
		font-size: 12px;
		color: var(--text3);
	}
	.appid-links {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.link-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		background: var(--bg3);
		border-radius: 8px;
		cursor: pointer;
		transition: background 0.15s;
		font-size: 13px;
	}
	.link-row:hover {
		background: var(--border);
	}
	.link-label {
		color: var(--text2);
		white-space: nowrap;
		min-width: 120px;
	}
	.link-url {
		flex: 1;
		color: var(--text);
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.copy-icon {
		color: var(--accent);
		flex-shrink: 0;
		opacity: 0.5;
		transition: opacity 0.15s;
	}
	.link-row:hover .copy-icon {
		opacity: 1;
	}
	@media (max-width: 767px) {
		.link-label {
			min-width: auto;
		}
	}
</style>

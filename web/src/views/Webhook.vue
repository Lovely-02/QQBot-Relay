<script setup>
	import { ref, computed, onMounted } from 'vue'
	import { useMessage } from 'naive-ui'
	import { http } from '../api'
	import SvgIcon from '../components/SvgIcon.vue'

	const message = useMessage()
	const forwards = ref([])
	const appids = ref([])
	const showCreate = ref(false)
	const form = ref({ appid: null, url: '' })

	const appidOptions = computed(() =>
		appids.value.map((a) => ({
			label: `${a.appid}${a.description ? ' — ' + a.description : ''}`,
			value: a.appid
		}))
	)

	const groupedForwards = computed(() => {
		const map = {}
		for (const fwd of forwards.value) {
			const id = fwd.appid || ''
			if (!map[id]) {
				const found = appids.value.find((a) => a.appid === id)
				map[id] = { appid: id, description: found?.description || '', targets: [] }
			}
			map[id].targets.push({ url: fwd.url })
		}
		return Object.values(map)
	})

	async function fetchData() {
		try {
			const [fwdRes, appidsRes] = await Promise.all([
				http.get('/api/admin/webhook/list'),
				http.get('/api/admin/appids')
			])
			forwards.value = fwdRes.data || []
			appids.value = appidsRes.data || []
		} catch {}
	}

	async function handleAdd() {
		if (!form.value.appid) {
			message.warning('请选择机器人')
			return
		}
		if (!form.value.url) {
			message.warning('请输入转发地址')
			return
		}
		try {
			await http.post('/api/admin/webhook/add', form.value)
			message.success('添加成功')
			form.value = { appid: null, url: '' }
			showCreate.value = false
			await fetchData()
		} catch (err) {
			message.error(err.response?.data?.error || '添加失败')
		}
	}

	async function handleRemove(appid, url) {
		try {
			await http.post('/api/admin/webhook/remove', { appid, url })
			message.success('已删除')
			await fetchData()
		} catch (err) {
			message.error(err.response?.data?.error || '删除失败')
		}
	}

	onMounted(fetchData)
</script>

<template>
	<div>
		<div class="section-header">
			<h3>二次转发配置</h3>
			<n-button type="primary" size="small" @click="showCreate = !showCreate">
				<template #icon>
					<SvgIcon :name="showCreate ? 'chevron-back' : 'plus'" :size="14" />
				</template>
				{{ showCreate ? '取消' : '新增' }}
			</n-button>
		</div>

		<p class="desc">将收到的 Webhook 消息按 AppID 二次转发到指定地址。</p>

		<n-card v-if="showCreate" :bordered="false" class="create-card" size="small">
			<div class="create-form">
				<n-select
					v-model:value="form.appid"
					:options="appidOptions"
					placeholder="选择机器人"
					size="small"
					filterable
					style="min-width: 160px"
				/>
				<n-input v-model:value="form.url" placeholder="转发目标 URL (http://...)" size="small" style="flex: 2" />
				<n-button type="primary" size="small" @click="handleAdd">
					<template #icon><SvgIcon name="plus" :size="14" /></template>
					添加
				</n-button>
			</div>
		</n-card>

		<div class="forward-list">
			<n-card v-for="group in groupedForwards" :key="group.appid" :bordered="false" class="forward-card">
				<div class="forward-header">
					<div class="forward-appid">
						<SvgIcon name="key" :size="16" />
						<code>{{ group.appid }}</code>
						<n-tag size="tiny" :bordered="false">{{ group.description || '' }}</n-tag>
					</div>
				</div>
				<div class="forward-targets">
					<div v-for="(target, idx) in group.targets" :key="idx" class="target-row">
						<SvgIcon name="external-link" :size="14" />
						<code class="target-url">{{ target.url }}</code>
						<n-popconfirm
							@positive-click="handleRemove(group.appid, target.url)"
							positive-text="删除"
							negative-text="取消"
						>
							<template #trigger>
								<n-button quaternary circle size="tiny" type="error" title="删除">
									<template #icon><SvgIcon name="trash" :size="13" /></template>
								</n-button>
							</template>
							确定要删除此转发？
						</n-popconfirm>
					</div>
				</div>
			</n-card>

			<n-empty v-if="!groupedForwards.length" description="暂无转发配置" />
		</div>
	</div>
</template>

<style scoped>
	.section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 8px;
	}
	.section-header h3 {
		font-size: 16px;
		font-weight: 600;
		color: var(--text);
		margin: 0;
	}
	.desc {
		font-size: 13px;
		color: var(--text3);
		margin: 0 0 16px;
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
		align-items: center;
	}
	.forward-list {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.forward-card {
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 10px;
		transition: box-shadow 0.2s;
	}
	.forward-card:hover {
		box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
	}
	.forward-header {
		margin-bottom: 10px;
	}
	.forward-appid {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.forward-appid code {
		font-size: 15px;
		font-weight: 600;
		color: var(--text);
	}
	.forward-targets {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.target-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		background: var(--bg3);
		border-radius: 8px;
		font-size: 13px;
	}
	.target-url {
		flex: 1;
		color: var(--text);
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>

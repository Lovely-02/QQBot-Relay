<script setup>
	import { ref, computed, onMounted, onUnmounted, h } from 'vue'
	import { NTag } from 'naive-ui'
	import { http } from '../api'
	import SvgIcon from '../components/SvgIcon.vue'

	const data = ref({})
	let timer = null

	const stats = computed(() => {
		const d = data.value
		const g = d.global || {}
		return [
			{
				label: '总消息',
				value: g.total_messages ?? 0,
				icon: 'chatbubbles',
				bg: 'linear-gradient(135deg, var(--success), #2ecc71)'
			},
			{
				label: 'WS 成功/失败',
				value: `${g.ws_success ?? 0} / ${g.ws_failure ?? 0}`,
				icon: 'zap',
				bg: 'linear-gradient(135deg, var(--info), var(--accent-light))'
			},
			{
				label: 'WH 成功/失败',
				value: `${g.wh_success ?? 0} / ${g.wh_failure ?? 0}`,
				icon: 'webhook',
				bg: 'linear-gradient(135deg, var(--warning), #f57731)'
			},
			{
				label: '在线连接',
				value: d.ws_online ?? 0,
				icon: 'people',
				bg: 'linear-gradient(135deg, var(--accent), var(--accent-light))'
			}
		]
	})

	async function fetchStats() {
		try {
			const { data: res } = await http.get('/api/admin/stats')
			data.value = res
		} catch {}
	}

	onMounted(() => {
		fetchStats()
		timer = setInterval(fetchStats, 5000)
	})
	onUnmounted(() => {
		if (timer) clearInterval(timer)
	})
</script>

<template>
	<div class="dash">
		<div class="banner">
			<h2>QQBot Relay</h2>
			<p>Webhook 到 WebSocket 桥接服务管理面板</p>
		</div>

		<div class="stat-grid">
			<div v-for="stat in stats" :key="stat.label" class="stat-card">
				<div class="stat-icon" :style="{ background: stat.bg }">
					<SvgIcon :name="stat.icon" :size="22" color="#fff" />
				</div>
				<div class="stat-value">{{ stat.value }}</div>
				<div class="stat-label">{{ stat.label }}</div>
			</div>
		</div>
	</div>
</template>

<style scoped>
	.dash {
		width: 100%;
	}
	.banner {
		background: linear-gradient(135deg, var(--accent), var(--accent-light));
		border-radius: 12px;
		padding: 24px 28px;
		margin-bottom: 20px;
	}
	.banner h2 {
		color: #fff;
		font-size: 20px;
		font-weight: 700;
		margin: 0 0 4px;
	}
	.banner p {
		color: rgba(255, 255, 255, 0.7);
		font-size: 13px;
		margin: 0;
	}
	.stat-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px;
		margin-bottom: 16px;
	}
	.stat-card {
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 16px;
		text-align: center;
	}
	.stat-icon {
		width: 40px;
		height: 40px;
		border-radius: 10px;
		margin: 0 auto 10px;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.stat-value {
		color: var(--text);
		font-size: 24px;
		font-weight: 700;
	}
	.stat-label {
		color: var(--text2);
		font-size: 12px;
		margin-top: 2px;
	}
	@media (max-width: 767px) {
		.stat-grid {
			grid-template-columns: repeat(2, 1fr);
		}
		.banner {
			padding: 16px 18px;
		}
	}
</style>

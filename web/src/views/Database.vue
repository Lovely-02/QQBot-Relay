<script setup>
	import { ref, computed, onMounted } from 'vue'
	import { useMessage } from 'naive-ui'
	import { http } from '../api'
	import SvgIcon from '../components/SvgIcon.vue'

	const message = useMessage()
	const tables = ref([])
	const currentTable = ref('')
	const tableData = ref(null)
	const loadingTables = ref(false)

	function formatCell(val) {
		if (val == null) return 'NULL'
		if (typeof val === 'object') return JSON.stringify(val)
		const s = String(val)
		return s.length > 200 ? s.slice(0, 200) + '...' : s
	}

	async function fetchTables() {
		loadingTables.value = true
		try {
			const { data } = await http.get('/api/admin/db/tables')
			tables.value = data
			if (data.length && !currentTable.value) {
				selectTable(data[0].table)
			}
		} catch {
			message.error('获取表列表失败')
		} finally {
			loadingTables.value = false
		}
	}

	function selectTable(name) {
		currentTable.value = name
		const tbl = tables.value.find((t) => t.table === name)
		if (tbl) {
			tableData.value = {
				table: tbl.table,
				total: tbl.row_count,
				columns: tbl.rows.length > 0 ? Object.keys(tbl.rows[0]) : [],
				rows: tbl.rows
			}
		}
	}

	onMounted(fetchTables)
</script>

<template>
	<div>
		<h3 class="page-heading">数据库查看</h3>

		<div class="table-tabs">
			<div
				v-for="tbl in tables"
				:key="tbl.table"
				:class="['table-tab', { active: currentTable === tbl.table }]"
				@click="selectTable(tbl.table)"
			>
				<SvgIcon name="server" :size="14" />
				<span>{{ tbl.table }}</span>
				<n-tag size="tiny" :bordered="false" round>{{ tbl.row_count }}</n-tag>
			</div>
		</div>

		<n-card v-if="tableData" :bordered="false" class="panel-card">
			<template #header>
				<div class="table-card-header">
					<span
						>{{ tableData.table }} <span class="row-count">({{ tableData.total }} 行)</span></span
					>
					<n-button quaternary circle size="small" @click="fetchTables" title="刷新">
						<template #icon><SvgIcon name="refresh" :size="16" /></template>
					</n-button>
				</div>
			</template>
			<div class="table-scroll">
				<table class="db-table">
					<thead>
						<tr>
							<th v-for="col in tableData.columns" :key="col">{{ col }}</th>
						</tr>
					</thead>
					<tbody>
						<tr v-for="(row, idx) in tableData.rows" :key="idx">
							<td v-for="col in tableData.columns" :key="col">
								<span class="cell-value" :title="formatCell(row[col])">{{ formatCell(row[col]) }}</span>
							</td>
						</tr>
					</tbody>
				</table>
			</div>
		</n-card>

		<n-empty v-else-if="!tables.length && !loadingTables" description="暂无数据表" />
	</div>
</template>

<style scoped>
	.page-heading {
		font-size: 16px;
		font-weight: 600;
		color: var(--text);
		margin-bottom: 16px;
	}
	.table-tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-bottom: 16px;
	}
	.table-tab {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 14px;
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 8px;
		cursor: pointer;
		font-size: 13px;
		color: var(--text2);
		transition: all 0.15s;
	}
	.table-tab:hover {
		background: var(--bg3);
		color: var(--text);
	}
	.table-tab.active {
		background: var(--accent);
		color: #fff;
		border-color: var(--accent);
	}
	.table-tab.active .n-tag {
		background: rgba(255, 255, 255, 0.2) !important;
		color: #fff !important;
	}
	.panel-card {
		background: var(--bg2);
		border: 1px solid var(--border);
		border-radius: 10px;
	}
	.table-card-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.row-count {
		font-size: 12px;
		color: var(--text3);
		font-weight: 400;
	}
	.table-scroll {
		overflow-x: auto;
	}
	.db-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 13px;
	}
	.db-table th {
		padding: 8px 12px;
		text-align: left;
		font-weight: 600;
		color: var(--text2);
		background: var(--bg3);
		border-bottom: 1px solid var(--border);
		white-space: nowrap;
	}
	.db-table td {
		padding: 6px 12px;
		border-bottom: 1px solid var(--border);
		color: var(--text);
		max-width: 300px;
	}
	.db-table tr:hover td {
		background: var(--bg3);
	}
	.cell-value {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 300px;
		font-family: Consolas, Monaco, monospace;
		font-size: 12px;
	}
	@media (max-width: 767px) {
		.cell-value {
			max-width: 150px;
		}
	}
</style>

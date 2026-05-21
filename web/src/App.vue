<script setup>
	import { computed } from 'vue'
	import { RouterView } from 'vue-router'
	import { NConfigProvider, NMessageProvider, NDialogProvider, NNotificationProvider } from 'naive-ui'
	import { zhCN, dateZhCN } from 'naive-ui'
	import { useThemeStore } from './stores/theme'

	const themeStore = useThemeStore()
	const themeOverrides = computed(() => themeStore.naiveOverrides(themeStore.theme))
</script>

<template>
	<n-config-provider :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN">
		<n-message-provider>
			<n-dialog-provider>
				<n-notification-provider>
					<router-view />
				</n-notification-provider>
			</n-dialog-provider>
		</n-message-provider>
	</n-config-provider>
</template>

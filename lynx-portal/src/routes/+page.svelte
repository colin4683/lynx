<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount, onDestroy } from 'svelte';
	import SystemStatusBadge from '$lib/components/SystemStatusBadge.svelte';
	import { getStatusConfig, type SystemStatus } from '$lib/utils/statusConfig';
	import { getSystemStatus, getMetricValue } from '$lib/utils/systemUtils';

	const { data } = $props();

	// State management
	let refreshInterval = $state<NodeJS.Timeout | null>(null);
	let lastRefresh = $state<Date>(new Date());

	// Derived computations for dashboard metrics
	const systemsOverview = $derived.by(() => {
		const total = data.systems.length;
		const statusCounts = data.systems.reduce((acc, system) => {
			const status = getSystemStatus(system);
			acc[status] = (acc[status] || 0) + 1;
			return acc;
		}, {} as Record<SystemStatus, number>);

		return {
			total,
			online: statusCounts.online || 0,
			offline: statusCounts.offline || 0,
			error: statusCounts.error || 0,
			warning: statusCounts.warning || 0,
			healthPercentage: total > 0 ? Math.round(((statusCounts.online || 0) / total) * 100) : 0
		};
	});

	const alertsOverview = $derived.by(() => {
		const totalAlerts = data.alerts.length;
		const criticalAlerts = data.alerts.filter(alert =>
			alert.alertRule?.severity === 'critical' || alert.alertRule?.severity === 'high'
		).length;
		const recentAlerts = data.alerts.filter(alert => {
			const alertTime = new Date(alert.date);
			const hourAgo = new Date(Date.now() - 60 * 60 * 1000);
			return alertTime > hourAgo;
		}).length;

		return {
			total: totalAlerts,
			critical: criticalAlerts,
			recent: recentAlerts
		};
	});

	const systemsNeedingAttention = $derived.by(() => {
		return data.systems
			.filter(system => {
				const status = getSystemStatus(system);
				return status === 'error' || status === 'warning';
			})
			.sort((a, b) => {
				const statusA = getSystemStatus(a);
				const statusB = getSystemStatus(b);
				const priorityA = getStatusConfig(statusA).priority;
				const priorityB = getStatusConfig(statusB).priority;
				return priorityB - priorityA; // Higher priority first
			})
			.slice(0, 5);
	});

	const resourceUtilization = $derived.by(() => {
		if (data.systems.length === 0) return { cpu: 0, memory: 0, disk: 0 };

		const totals = data.systems.reduce((acc, system) => {
			acc.cpu += getMetricValue(system, 'cpu');
			acc.memory += getMetricValue(system, 'memory');
			acc.disk += getMetricValue(system, 'disk');
			return acc;
		}, { cpu: 0, memory: 0, disk: 0 });

		return {
			cpu: Math.round(totals.cpu / data.systems.length),
			memory: Math.round(totals.memory / data.systems.length),
			disk: Math.round(totals.disk / data.systems.length)
		};
	});

	const recentActivity = $derived.by(() => {
		// Combine recent alerts and system status changes
		const activities = [
			...data.alerts.slice(0, 5).map(alert => ({
				type: 'alert' as const,
				title: alert.alertRule?.name || 'Unknown Alert',
				system: alert.system?.label || 'Unknown System',
				timestamp: alert.date,
				severity: alert.alertRule?.severity || 'info'
			})),
			...data.systems
				.filter(system => system.lastSeen)
				.sort((a, b) => new Date(b.lastSeen!).getTime() - new Date(a.lastSeen!).getTime())
				.slice(0, 3)
				.map(system => ({
					type: 'system' as const,
					title: 'System Update',
					system: system.label,
					timestamp: system.lastSeen!,
					severity: getSystemStatus(system)
				}))
		]
		.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
		.slice(0, 8);

		return activities;
	});

	const systemsOverviewList = $derived.by(() => {
		return data.systems
			.sort((a, b) => {
				// Sort by status priority (errors first, then warnings, then online, then offline)
				const statusA = getSystemStatus(a);
				const statusB = getSystemStatus(b);
				const priorityA = getStatusConfig(statusA).priority;
				const priorityB = getStatusConfig(statusB).priority;
				if (priorityA !== priorityB) {
					return priorityA - priorityB;
				}
				// Then sort by label alphabetically
				return a.label.localeCompare(b.label);
			})
			.slice(0, 6); // Show top 6 systems
	});

	// Utility functions
	function relativeDate(date: string): string {
		const now = new Date();
		const diff = now.getTime() - new Date(date).getTime();
		const seconds = Math.floor(diff / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);

		if (days > 0) return `${days} day${days > 1 ? 's' : ''} ago`;
		if (hours > 0) return `${hours} hour${hours > 1 ? 's' : ''} ago`;
		if (minutes > 0) return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
		return `${seconds} second${seconds > 1 ? 's' : ''} ago`;
	}

	function getHealthColor(percentage: number): string {
		if (percentage >= 80) return 'var(--primary)';
		if (percentage >= 60) return 'var(--disk)';
		return 'var(--memory)';
	}

	function getUtilizationColor(percentage: number): string {
		if (percentage < 70) return 'var(--primary)';
		if (percentage < 85) return 'var(--disk)';
		return 'var(--memory)';
	}

	function navigateToSystems() {
		goto('/systems');
	}

	function navigateToSystemsWithFilter(status: string) {
		goto(`/systems?status=${status}`);
	}

	function navigateToAlerts() {
		goto('/alerts');
	}

	function navigateToSystemDetail(systemId: number) {
		goto(`/systems/${systemId}`);
	}

	// Real-time updates
	function startPolling() {
		refreshInterval = setInterval(() => {
			lastRefresh = new Date();
			// In a real app, you would make an API call to refresh data
			// For now, we'll just reload the page to get fresh data
			window.location.reload();
		}, 60000); // 60 seconds
	}

	onMount(() => {
		startPolling();
	});

	onDestroy(() => {
		if (refreshInterval) {
			clearInterval(refreshInterval);
		}
	});
</script>

<div class="w-full space-y-6">
	<!-- Page Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-3xl font-bold text-[var(--text)]">Dashboard</h1>
			<p class="text-[var(--text)]/70 mt-1">
				Monitor your infrastructure at a glance
			</p>
		</div>
		<div class="text-sm text-[var(--text)]/60">
			Last updated: {lastRefresh.toLocaleTimeString()}
		</div>
	</div>

	{#if data.systems.length === 0}
		<!-- Empty State -->
		<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-12 text-center">
			<div class="mx-auto w-24 h-24 mb-6 opacity-50">
				<span class="icon-[heroicons--server] w-full h-full text-[var(--text)]"></span>
			</div>
			<h2 class="text-xl font-semibold text-[var(--text)] mb-3">Welcome to Lynx</h2>
			<p class="text-[var(--text)]/70 mb-6 max-w-md mx-auto">
				Get started by deploying your first monitoring agent to begin tracking your infrastructure.
			</p>
			<button
				onclick={() => goto('/systems')}
				class="inline-flex items-center gap-2 px-6 py-3 bg-[var(--primary)] text-black font-medium rounded-md hover:opacity-90 transition-opacity"
			>
				<span class="icon-[heroicons--plus] w-5 h-5"></span>
				Deploy First Agent
			</button>
		</div>
	{:else}
		<!-- Dashboard Grid -->
		<div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
			<!-- Top Row - Critical Status -->
			<div class="lg:col-span-12">
				<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
					<!-- Overall Health -->
					<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6">
						<div class="flex items-center justify-between mb-4">
							<h3 class="text-sm font-medium text-[var(--text)]/70">Infrastructure Health</h3>
							<span class="icon-[heroicons--heart] w-5 h-5" style="color: {getHealthColor(systemsOverview.healthPercentage)}"></span>
						</div>
						<div class="text-3xl font-bold mb-2" style="color: {getHealthColor(systemsOverview.healthPercentage)}">
							{systemsOverview.healthPercentage}%
						</div>
						<div class="text-sm text-[var(--text)]/60">
							{systemsOverview.online} of {systemsOverview.total} systems online
						</div>
					</div>

					<!-- Critical Alerts -->
					<button
						class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6 cursor-pointer hover:bg-[var(--background-alt)] transition-colors text-left"
						onclick={navigateToAlerts}
					>
						<div class="flex items-center justify-between mb-4">
							<h3 class="text-sm font-medium text-[var(--text)]/70">Critical Issues</h3>
							<span class="icon-[heroicons--exclamation-triangle] w-5 h-5 text-[var(--memory)]"></span>
						</div>
						<div class="text-3xl font-bold text-[var(--memory)] mb-2">
							{alertsOverview.critical}
						</div>
						<div class="text-sm text-[var(--text)]/60">
							{alertsOverview.recent} in last hour
						</div>
					</button>

					<!-- Systems Status -->
					<button
						class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6 cursor-pointer hover:bg-[var(--background-alt)] transition-colors text-left"
						onclick={navigateToSystems}
					>
						<div class="flex items-center justify-between mb-4">
							<h3 class="text-sm font-medium text-[var(--text)]/70">Systems</h3>
							<span class="icon-[heroicons--server-stack] w-5 h-5 text-[var(--primary)]"></span>
						</div>
						<div class="text-3xl font-bold text-[var(--text)] mb-2">
							{systemsOverview.total}
						</div>
						<div class="flex gap-3 text-sm">
							<span class="text-[var(--primary)]">{systemsOverview.online} up</span>
							<span class="text-[var(--text)]/60">{systemsOverview.offline} down</span>
						</div>
					</button>

					<!-- Resource Usage -->
					<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6">
						<div class="flex items-center justify-between mb-4">
							<h3 class="text-sm font-medium text-[var(--text)]/70">Avg. Resources</h3>
							<span class="icon-[heroicons--cpu-chip] w-5 h-5 text-[var(--cpu)]"></span>
						</div>
						<div class="space-y-3">
							<div class="flex items-center justify-between">
								<span class="text-sm text-[var(--text)]/70">CPU</span>
								<span class="text-sm font-medium" style="color: {getUtilizationColor(resourceUtilization.cpu)}">
									{resourceUtilization.cpu}%
								</span>
							</div>
							<div class="flex items-center justify-between">
								<span class="text-sm text-[var(--text)]/70">Memory</span>
								<span class="text-sm font-medium" style="color: {getUtilizationColor(resourceUtilization.memory)}">
									{resourceUtilization.memory}%
								</span>
							</div>
							<div class="flex items-center justify-between">
								<span class="text-sm text-[var(--text)]/70">Disk</span>
								<span class="text-sm font-medium" style="color: {getUtilizationColor(resourceUtilization.disk)}">
									{resourceUtilization.disk}%
								</span>
							</div>
						</div>
					</div>
				</div>
			</div>

			<!-- Second Row - Systems Needing Attention -->
			{#if systemsNeedingAttention.length > 0}
			<div class="lg:col-span-6">
				<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6 h-full">
					<div class="flex items-center justify-between mb-6">
						<h3 class="text-lg font-semibold text-[var(--text)]">Systems Needing Attention</h3>
						<button
							onclick={() => navigateToSystemsWithFilter('error,warning')}
							class="text-sm text-[var(--primary)] hover:underline"
						>
							View All
						</button>
					</div>
					<div class="space-y-4">
						{#each systemsNeedingAttention as system (system.id)}
							<button
								class="w-full flex items-center justify-between p-4 bg-[var(--background)] rounded-lg border border-[var(--border)] cursor-pointer hover:bg-[var(--background-alt)] transition-colors text-left"
								onclick={() => navigateToSystemDetail(system.id)}
							>
								<div class="flex items-center gap-3">
									<SystemStatusBadge status={getSystemStatus(system)} variant="compact" size="sm" />
									<div>
										<div class="font-medium text-[var(--text)]">{system.label}</div>
										<div class="text-sm text-[var(--text)]/60">
											Last seen: {system.lastSeen ? relativeDate(system.lastSeen) : 'Never'}
										</div>
									</div>
								</div>
								<div class="flex flex-col gap-2">
									<div class="font-mono text-sm">
										<span>Alert: {system.alertHistories[0].alertRule.name}</span>
									</div>
									<div class="flex items-center gap-2 font-mono text-sm text-[var(--text)]/60">
										<span>CPU: {getMetricValue(system, 'cpu').toFixed(0)}%</span>
										<span>•</span>
										<span>RAM: {getMetricValue(system, 'memory').toFixed(0)}%</span>
									</div>
								</div>
							</button>
						{/each}
					</div>
				</div>
			</div>
			{/if}


			<!-- Systems Overview -->
			<div class="lg:col-span-12">
				<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6">
					<div class="flex items-center justify-between mb-6">
						<h3 class="text-lg font-semibold text-[var(--text)]">Systems Overview</h3>
						<button
							onclick={navigateToSystems}
							class="text-sm text-[var(--primary)] hover:underline"
						>
							View All
						</button>
					</div>

					{#if systemsOverviewList.length === 0}
						<div class="text-center py-8 text-[var(--text)]/60">
							<span class="icon-[heroicons--server] w-8 h-8 mx-auto mb-2 opacity-50"></span>
							<p>No systems configured</p>
						</div>
					{:else}
						<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
							{#each systemsOverviewList as system (system.id)}
								<button
									class="bg-[var(--background)] rounded-lg border border-[var(--border)] p-4 text-left hover:bg-[var(--background-alt)] transition-colors"
									onclick={() => navigateToSystemDetail(system.id)}
								>
									<div class="flex items-start justify-between mb-3">
										<div class="flex items-center gap-2">
											<SystemStatusBadge status={getSystemStatus(system)} variant="dot-only" size="sm" />
											<div class="font-medium text-[var(--text)] truncate">{system.label}</div>
										</div>
										<div class="text-xs text-[var(--text)]/40 ml-2">
											{system.lastSeen ? relativeDate(system.lastSeen) : 'Never'}
										</div>
									</div>

									<div class="space-y-2 text-sm">
										<div class="flex items-center justify-between">
											<span class="text-[var(--text)]/60">Hostname:</span>
											<span class="text-[var(--text)] font-mono text-xs truncate max-w-32" title={system.hostname || 'Unknown'}>
												{system.hostname || 'Unknown'}
											</span>
										</div>

										<div class="flex items-center justify-between">
											<span class="text-[var(--text)]/60">OS:</span>
											<span class="text-[var(--text)] text-xs truncate max-w-32" title={system.os || 'Unknown'}>
												{system.os || 'Unknown'}
											</span>
										</div>

										<div class="flex items-center justify-between">
											<span class="text-[var(--text)]/60">CPU:</span>
											<span class="text-[var(--text)] text-xs truncate max-w-32" title={system.cpu || 'Unknown'}>
												{system.cpu || 'Unknown'}
											</span>
										</div>

										<div class="flex items-center justify-between">
											<span class="text-[var(--text)]/60">Cores:</span>
											<span class="text-[var(--text)] text-xs">
												{system.cpuCount || 'N/A'}
											</span>
										</div>

										<div class="flex items-center justify-between">
											<span class="text-[var(--text)]/60">Memory:</span>
											<span class="text-[var(--text)] text-xs">
												{#if system.memoryTotal}
													{Math.round(system.memoryTotal / (1024 * 1024))} GB
												{:else}
													N/A
												{/if}
											</span>
										</div>
									</div>

									<!-- Resource indicators -->
									<div class="mt-3 pt-3 border-t border-[var(--border)] flex justify-between text-xs">
										<div class="flex items-center gap-1">
											<div class="w-2 h-2 rounded-full bg-[var(--cpu)]"></div>
											<span class="text-[var(--text)]/60">CPU:</span>
											<span style="color: {getUtilizationColor(getMetricValue(system, 'cpu'))}">{getMetricValue(system, 'cpu').toFixed(0)}%</span>
										</div>
										<div class="flex items-center gap-1">
											<div class="w-2 h-2 rounded-full bg-[var(--memory)]"></div>
											<span class="text-[var(--text)]/60">RAM:</span>
											<span style="color: {getUtilizationColor(getMetricValue(system, 'memory'))}">{getMetricValue(system, 'memory').toFixed(0)}%</span>
										</div>
										<div class="flex items-center gap-1">
											<div class="w-2 h-2 rounded-full bg-[var(--disk)]"></div>
											<span class="text-[var(--text)]/60">Disk:</span>
											<span style="color: {getUtilizationColor(getMetricValue(system, 'disk'))}">{getMetricValue(system, 'disk').toFixed(0)}%</span>
										</div>
									</div>
								</button>
							{/each}
						</div>

						{#if data.systems.length > 6}
							<div class="text-center mt-6">
								<button
									onclick={navigateToSystems}
									class="inline-flex items-center gap-2 px-4 py-2 text-sm bg-[var(--primary)] text-black font-medium rounded-md hover:opacity-90 transition-opacity"
								>
									View All {data.systems.length} Systems
									<span class="icon-[heroicons--arrow-right] w-4 h-4"></span>
								</button>
							</div>
						{/if}
					{/if}
				</div>
			</div>

			<!-- Recent Activity -->
			<div class="lg:col-span-6">
				<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6 h-full">
					<div class="flex items-center justify-between mb-6">
						<h3 class="text-lg font-semibold text-[var(--text)]">Recent Activity</h3>
						<button
							onclick={navigateToAlerts}
							class="text-sm text-[var(--primary)] hover:underline"
						>
							View All
						</button>
					</div>
					<div class="space-y-3 max-h-80 overflow-y-auto">
						{#each recentActivity as activity (activity.timestamp + activity.title)}
							<div class="flex items-start gap-3 p-3 bg-[var(--background)] rounded-lg border border-[var(--border)]">
								<div class="flex-shrink-0 mt-1">
									{#if activity.type === 'alert'}
										<span class="icon-[heroicons--exclamation-triangle] w-4 h-4 text-[var(--memory)]"></span>
									{:else}
										<span class="icon-[heroicons--arrow-path] w-4 h-4 text-[var(--primary)]"></span>
									{/if}
								</div>
								<div class="flex-1 min-w-0">
									<div class="text-sm font-medium text-[var(--text)] truncate">
										{activity.title}
									</div>
									<div class="text-sm text-[var(--text)]/60 truncate">
										{activity.system}
									</div>
									<div class="text-xs text-[var(--text)]/40 mt-1">
										{relativeDate(activity.timestamp)}
									</div>
								</div>
							</div>
						{/each}
						{#if recentActivity.length === 0}
							<div class="text-center py-8 text-[var(--text)]/60">
								<span class="icon-[heroicons--clock] w-8 h-8 mx-auto mb-2 opacity-50"></span>
								<p>No recent activity</p>
							</div>
						{/if}
					</div>
				</div>
			</div>

		</div>
	{/if}
</div>
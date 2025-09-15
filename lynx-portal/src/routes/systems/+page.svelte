<script lang="ts">
	import ProgressBar from '$lib/ProgressBar.svelte';
	import SystemStatusBadge from '$lib/components/SystemStatusBadge.svelte';
	import { goto } from '$app/navigation';
	import { getSystemStatus, getMetricValue, type SystemStatus } from '$lib/utils/systemUtils';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import { onMount } from 'svelte';

	const { data } = $props();

	// State management using Svelte 5 runes
	let searchTerm = $state('');
	let statusFilter = $state(["all"]); // 'all', 'online', 'offline', 'error', 'warning', 'maintenance'
	let viewMode = $state('grid'); // 'grid' or 'table'
	let sortField = $state('label');
	let sortDirection = $state('asc');
	let refreshInterval = $state<number | null>(null);

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

	function navigateToSystem(systemId: number) {
		goto(`/systems/${systemId}`);
	}

	function toggleSort(field: string) {
		if (sortField === field) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			sortField = field;
			sortDirection = 'asc';
		}
	}

	// Derived computations
	const filteredAndSortedSystems = $derived.by(() => {
		let filtered = data.systems.filter(system => {
			// Fix the search logic with better null safety
			const searchLower = searchTerm.toLowerCase();
			const labelMatch = system.label ? system.label.toLowerCase().includes(searchLower) : false;
			const hostnameMatch = system.hostname ? system.hostname.toLowerCase().includes(searchLower) : false;
			const matchesSearch = searchTerm === '' || labelMatch || hostnameMatch;

			const systemStatus = getSystemStatus(system);
			const matchesStatus = statusFilter.length && statusFilter[0] === 'all' || statusFilter.includes(systemStatus);

			return matchesSearch && matchesStatus;
		});

		return filtered.sort((a, b) => {
			let aValue, bValue;

			switch (sortField) {
				case 'label':
					aValue = a.label?.toLowerCase() || '';
					bValue = b.label?.toLowerCase() || '';
					break;
				case 'status':
					aValue = getSystemStatus(a);
					bValue = getSystemStatus(b);
					break;
				case 'cpu':
					aValue = getMetricValue(a, 'cpu');
					bValue = getMetricValue(b, 'cpu');
					break;
				case 'memory':
					aValue = getMetricValue(a, 'memory');
					bValue = getMetricValue(b, 'memory');
					break;
				case 'disk':
					aValue = getMetricValue(a, 'disk');
					bValue = getMetricValue(b, 'disk');
					break;
				case 'lastSeen':
					aValue = new Date(a.lastSeen || 0).getTime();
					bValue = new Date(b.lastSeen || 0).getTime();
					break;
				default:
					return 0;
			}

			if (aValue < bValue) return sortDirection === 'asc' ? -1 : 1;
			if (aValue > bValue) return sortDirection === 'asc' ? 1 : -1;
			return 0;
		});
	});

	const summaryStats = $derived.by(() => {
		const stats = data.systems.reduce((acc, system) => {
			const status = getSystemStatus(system);
			acc.total++;
			acc[status]++;
			return acc;
		}, { total: 0, online: 0, offline: 0, error: 0, warning: 0 });

		return stats;
	});

	// Effects for real-time updates
	$effect(() => {
		refreshInterval = setInterval(() => {
			// In a real app, you'd refresh the data here
			window.location.reload();
		}, 30000); // 30 seconds

		return () => {
			if (refreshInterval) {
				clearInterval(refreshInterval);
			}
		};
	});

	onMount(() => {
		// retrieve filters from url params if needed
		const params = new URLSearchParams(window.location.search);
		if (params.get('view')) {
			viewMode = params.get('view') === 'table' ? 'table' : 'grid';
		}
		if (params.get('status')) {
			const status = params.get('status');
			if (!status) return;
			const validStatuses = ['all', 'online', 'offline', 'error', 'warning', 'maintenance'];
			const statusArray = status.split(',').filter(s => validStatuses.includes(s));
			statusFilter = statusArray.length ? statusArray : ['all'];
		}
	})

</script>

<div class="flex flex-col gap-6 w-full px-6 py-4">
	<!-- Summary Status Bar -->
	<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-6">
		<h2 class="text-xl font-semibold mb-4 text-[var(--text)]">Systems Overview</h2>
		<div class="flex items-center justify-around space-x-6">
			<div class="text-center">
				<div class="text-2xl font-bold text-[var(--text)]">{summaryStats.total}</div>
				<div class="text-sm text-[var(--text)] opacity-70">Total Systems</div>
			</div>
			<div class="text-center">
				<div class="text-2xl font-bold text-[var(--primary)]">{summaryStats.online}</div>
				<div class="text-sm text-[var(--text)] opacity-70">Online</div>
			</div>
			<div class="text-center">
				<div class="text-2xl font-bold text-[var(--text)] opacity-60">{summaryStats.offline}</div>
				<div class="text-sm text-[var(--text)] opacity-70">Offline</div>
			</div>
			<div class="text-center">
				<div class="text-2xl font-bold text-[var(--memory)]">{summaryStats.error}</div>
				<div class="text-sm text-[var(--text)] opacity-70">Errors</div>
			</div>
			<div class="text-center">
				<div class="text-2xl font-bold text-[var(--disk)]">{summaryStats.warning}</div>
				<div class="text-sm text-[var(--text)] opacity-70">Warnings</div>
			</div>
		</div>
	</div>

	<!-- Search and Filter Bar -->
	<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-4">
		<div class="flex flex-col md:flex-row gap-4 items-center justify-between">
			<div class="flex flex-col align-middle items-center md:flex-row gap-4 flex-1">
				<div class="relative flex-1">
					<span class="icon-[heroicons--magnifying-glass] absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-[var(--text)] opacity-50"></span>
					<input
						type="text"
						placeholder="Search systems..."
						bind:value={searchTerm}
						class="w-full pl-10 pr-4 py-2 bg-[var(--background)] border border-[var(--border)] rounded-lg text-[var(--text)] placeholder:text-[var(--text)] placeholder:opacity-50 focus:border-[var(--primary)] focus:outline-none"
					/>
				</div>
				<Select.Root  type="multiple" bind:value={statusFilter}>
					<Select.Trigger class="flex w-[180px] items-center gap-0 align-middle">
								<span class="flex items-center gap-2">
									<span class="text-sm">{statusFilter}</span>
								</span>
					</Select.Trigger>
					<Select.Content
						class="rounded-md border border-[var(--border)] bg-[var(--background)]"
					>
						<Select.Item value="all">All Status</Select.Item>
						<Select.Item value="online">Online</Select.Item>
						<Select.Item value="offline">Offline</Select.Item>
						<Select.Item value="error">Error</Select.Item>
						<Select.Item value="warning">Warning</Select.Item>
						<Select.Item value="maintenance">Maintenance</Select.Item>
					</Select.Content>
				</Select.Root>
			</div>
			<div class="flex items-center gap-2">
				<span class="text-sm text-[var(--text)] opacity-70">View:</span>
				<Button
					onclick={() => viewMode = 'grid'}
					class="cursor-pointer p-2 rounded-lg transition-colors {viewMode === 'grid' ? 'bg-primary/60 border border-primary text-[var(--primary)]' : 'text-[var(--text)] bg-foreground border-border opacity-60 hover:opacity-80'}"
				>
					<span class="icon-[heroicons--squares-2x2] w-4 h-4"></span>
				</Button>
				<Button
					onclick={() => viewMode = 'table'}
					class="cursor-pointer p-2 rounded-lg transition-colors {viewMode === 'table' ? 'bg-primary border border-primary  text-[var(--primary)]' : 'text-[var(--text)] bg-foreground border-border opacity-60 hover:opacity-80'}"
				>
					<span class="icon-[heroicons--list-bullet] w-4 h-4"></span>
				</Button>
			</div>
		</div>
	</div>

	<!-- Systems Display -->
	{#if filteredAndSortedSystems.length === 0}
		<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] p-12 text-center">
			<span class="icon-[heroicons--server] w-16 h-16 text-[var(--text)] opacity-30 mx-auto mb-4"></span>
			<h3 class="text-lg font-medium text-[var(--text)] mb-2">No systems found</h3>
			<p class="text-[var(--text)] opacity-70 mb-4">
				{searchTerm || statusFilter.includes("all") ? 'Try adjusting your filters.' : 'Deploy your first agent to get started.'}
			</p>
			{#if !searchTerm && statusFilter.includes("all")}
				<button class="bg-[var(--primary)] text-[var(--background)] px-4 py-2 rounded-lg hover:bg-[var(--primary)] hover:opacity-90 transition-opacity font-medium">
					Add System
				</button>
			{/if}
		</div>
	{:else if viewMode === 'grid'}
		<!-- Grid View -->
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
			{#each filteredAndSortedSystems as system}
				<div
					class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] hover:border-[var(--primary)] hover:border-opacity-50 transition-all cursor-pointer"
					onclick={() => navigateToSystem(system.id)}
				>
					<!-- Card Header -->
					<div class="p-4 border-b border-[var(--border)]">
						<div class="flex items-center justify-between mb-2">
							<h3 class="font-semibold text-[var(--text)] truncate">{system.label}</h3>
							<SystemStatusBadge status={getSystemStatus(system)} />
						</div>
						{#if system.os}
							<p class="text-sm text-[var(--text)] opacity-70 truncate">{system.os}</p>
						{/if}
						{#if !system.os && !system.cpu}
							<p class="text-sm text-[var(--text)] opacity-70 truncate">No information available</p>
						{/if}
					</div>

					<!-- Card Body -->
					<div class="p-4 space-y-3">
						<!-- CPU -->
						<div>
							<div class="flex justify-between items-center mb-1">
								<span class="text-xs text-[var(--text)] opacity-70">CPU</span>
								<span class="text-xs font-medium text-[var(--text)]">
									{system.cpuUsage ? system.cpuUsage.toFixed(1) + '%' : 'N/A'}
								</span>
							</div>
							<div class="w-full bg-[var(--background)] rounded-full h-2">
								<div class="h-2 rounded-full bg-[var(--cpu)]" style="width: {Math.min(getMetricValue(system, 'cpu'), 100)}%"></div>
							</div>
						</div>

						<!-- Memory -->
						<div>
							<div class="flex justify-between items-center mb-1">
								<span class="text-xs text-[var(--text)] opacity-70">Memory</span>
								<span class="text-xs font-medium text-[var(--text)]">
									{getMetricValue(system, 'memory') > 0 ? getMetricValue(system, 'memory').toFixed(1) + '%' : 'N/A'}
								</span>
							</div>
							<div class="w-full bg-[var(--background)] rounded-full h-2">
								<div class="h-2 rounded-full bg-[var(--memory)]" style="width: {Math.min(getMetricValue(system, 'memory'), 100)}%"></div>
							</div>
						</div>

						<!-- Disk -->
						<div>
							<div class="flex justify-between items-center mb-1">
								<span class="text-xs text-[var(--text)] opacity-70">Disk</span>
								<span class="text-xs font-medium text-[var(--text)]">
									{getMetricValue(system, 'disk') > 0 ? getMetricValue(system, 'disk').toFixed(1) + '%' : 'N/A'}
								</span>
							</div>
							<div class="w-full bg-[var(--background)] rounded-full h-2">
								<div class="h-2 rounded-full bg-[var(--disk)]" style="width: {Math.min(getMetricValue(system, 'disk'), 100)}%"></div>
							</div>
						</div>
					</div>

					<!-- Card Footer -->
					<div class="px-4 py-3  border-t border-[var(--border)] bg-[var(--background)] rounded-b-lg">
						<div class="flex justify-between items-center text-xs text-[var(--text)] opacity-70">
							<span>Last seen:</span>
							<span>{system.lastSeen ? relativeDate(system.lastSeen) : 'Never'}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<!-- Table View -->
		<div class="bg-[var(--foreground)] rounded-lg border border-[var(--border)] overflow-hidden">
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead class="bg-[var(--background)]">
						<tr>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider cursor-pointer hover:bg-[var(--background-alt)] transition-colors"
								onclick={() => toggleSort('status')}>
								Status
								{#if sortField === 'status'}
									<span class="icon-[heroicons--chevron-{sortDirection === 'asc' ? 'up' : 'down'}] w-3 h-3 inline ml-1"></span>
								{/if}
							</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider cursor-pointer hover:bg-[var(--background-alt)] transition-colors"
								onclick={() => toggleSort('label')}>
								System
								{#if sortField === 'label'}
									<span class="icon-[heroicons--chevron-{sortDirection === 'asc' ? 'up' : 'down'}] w-3 h-3 inline ml-1"></span>
								{/if}
							</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider cursor-pointer hover:bg-[var(--background-alt)] transition-colors"
								onclick={() => toggleSort('cpu')}>
								CPU
								{#if sortField === 'cpu'}
									<span class="icon-[heroicons--chevron-{sortDirection === 'asc' ? 'up' : 'down'}] w-3 h-3 inline ml-1"></span>
								{/if}
							</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider cursor-pointer hover:bg-[var(--background-alt)] transition-colors"
								onclick={() => toggleSort('memory')}>
								Memory
								{#if sortField === 'memory'}
									<span class="icon-[heroicons--chevron-{sortDirection === 'asc' ? 'up' : 'down'}] w-3 h-3 inline ml-1"></span>
								{/if}
							</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider cursor-pointer hover:bg-[var(--background-alt)] transition-colors"
								onclick={() => toggleSort('disk')}>
								Disk
								{#if sortField === 'disk'}
									<span class="icon-[heroicons--chevron-{sortDirection === 'asc' ? 'up' : 'down'}] w-3 h-3 inline ml-1"></span>
								{/if}
							</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider cursor-pointer hover:bg-[var(--background-alt)] transition-colors"
								onclick={() => toggleSort('lastSeen')}>
								Last Seen
								{#if sortField === 'lastSeen'}
									<span class="icon-[heroicons--chevron-{sortDirection === 'asc' ? 'up' : 'down'}] w-3 h-3 inline ml-1"></span>
								{/if}
							</th>
							<th class="px-6 py-3 text-left text-xs font-medium text-[var(--text)] opacity-70 uppercase tracking-wider">
								Actions
							</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-[var(--border)]">
						{#each filteredAndSortedSystems as system}
							<tr class="hover:bg-[var(--background)] transition-colors cursor-pointer" onclick={() => navigateToSystem(system.id)}>
								<td class="px-6 py-4 whitespace-nowrap">
									<SystemStatusBadge status={getSystemStatus(system)} size="sm" />
								</td>
								<td class="px-6 py-4 whitespace-nowrap">
									<div>
										<div class="text-sm font-medium text-[var(--text)]">{system.label}</div>
										{#if system.hostname}
											<div class="text-sm text-[var(--text)] opacity-70">{system.hostname}</div>
										{/if}
									</div>
								</td>
								<td class="px-6 py-4 whitespace-nowrap">
									<div class="flex items-center gap-2">
										<span class="text-sm text-[var(--text)] w-12">
											{system.cpuUsage ? system.cpuUsage.toFixed(1) + '%' : 'N/A'}
										</span>
										<div class="w-16">
											<ProgressBar min={0} max={1} value={getMetricValue(system, 'cpu') / 100} type="cpu" />
										</div>
									</div>
								</td>
								<td class="px-6 py-4 whitespace-nowrap">
									<div class="flex items-center gap-2">
										<span class="text-sm text-[var(--text)] w-12">
											{getMetricValue(system, 'memory') > 0 ? getMetricValue(system, 'memory').toFixed(1) + '%' : 'N/A'}
										</span>
										<div class="w-16">
											<ProgressBar min={0} max={1} value={getMetricValue(system, 'memory') / 100} type="memory" />
										</div>
									</div>
								</td>
								<td class="px-6 py-4 whitespace-nowrap">
									<div class="flex items-center gap-2">
										<span class="text-sm text-[var(--text)] w-12">
											{getMetricValue(system, 'disk') > 0 ? getMetricValue(system, 'disk').toFixed(1) + '%' : 'N/A'}
										</span>
										<div class="w-16">
											<ProgressBar min={0} max={1} value={getMetricValue(system, 'disk') / 100} type="disk" />
										</div>
									</div>
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-[var(--text)]">
									{system.lastSeen ? relativeDate(system.lastSeen) : 'Never'}
								</td>
								<td class="px-6 py-4 whitespace-nowrap text-sm text-[var(--text)] opacity-70">
									<div class="flex items-center gap-2">
										<button class="text-[var(--text)] opacity-60 hover:opacity-80 transition-opacity">
											<span class="icon-[heroicons--cog-6-tooth] w-4 h-4"></span>
										</button>
										<button class="text-[var(--text)] opacity-60 hover:opacity-80 transition-opacity">
											<span class="icon-[heroicons--ellipsis-horizontal] w-4 h-4"></span>
										</button>
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}
</div>

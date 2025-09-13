<script lang="ts">
	import { Switch } from "$lib/components/ui/switch/index.js";
	import { Button } from '$lib/components/ui/button';
	import * as Select from "$lib/components/ui/select/index.js";
	import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
	import { toast } from 'svelte-sonner';

	const { data } = $props();

	let alerts = $derived.by(() => {
		return data.alerts ?? [];
	});

	const systems = $derived.by(() => {
		return data.systems ?? [];
	});


	// State for managing alert operations
	let alertStates = $state<Record<number, { active: boolean; isToggling: boolean; isDeleting: boolean }>>({});

	// Initialize alert states
	$effect(() => {
		alerts.forEach(alert => {
			if (!alertStates[alert.id]) {
				alertStates[alert.id] = {
					active: alert.active ?? false,
					isToggling: false,
					isDeleting: false
				};
			}
		});
	});

	// Group systems by alert rule (since rules can be reused across systems)
	const alertsWithSystems = $derived.by(() => {
		return alerts.map(alert => ({
			...alert,
			systems: alert.alertSystems?.map(as => as.system) || []
		}));
	});

	// Quick actions
	async function toggleAlert(alertId: number) {
		if (!alertStates[alertId]) return;

		alertStates[alertId].isToggling = true;
		try {
			// Simulate API call - in real implementation, this would be an actual API call
			let res = await fetch('/alerts', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ id: alertId, active: !alertStates[alertId].active })
			});
			if (!res.ok) {
				toast.error("Failed to enable/disable alert.");
			} else {
				alertStates[alertId].active = !alertStates[alertId].active;
			}
		} catch (error) {
			console.error('Failed to toggle alert:', error);
		} finally {
			alertStates[alertId].isToggling = false;
		}
	}

	async function deleteAlert(alertId: number) {
		if (!alertStates[alertId] || !confirm('Are you sure you want to delete this alert rule? This action cannot be undone.')) return;

		alertStates[alertId].isDeleting = true;
		try {
			// Simulate API call - in real implementation, this would be an actual API call
			let res = await fetch('/alerts', {
				method: 'DELETE',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ id: alertId })
			});
			if (!res.ok) {
				toast.error("Failed to delete alert.");
				return;
			}
			alerts = alerts.filter(alert => alert.id !== alertId);
			toast.success("Alert deleted successfully.");
		} catch (error) {
			console.error('Failed to delete alert:', error);
		} finally {
			alertStates[alertId].isDeleting = false;
		}
	}

	function viewHistory(alertId: number) {
		window.location.href = `/alerts/history/${alertId}`;
	}

	function getSeverityColor(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'text-red-500';
			case 'high': return 'text-orange-500';
			case 'medium': return 'text-yellow-500';
			case 'low': return 'text-green-500';
			default: return 'text-gray-500';
		}
	}

	function getSeverityIcon(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'icon-[heroicons--exclamation-triangle-solid]';
			case 'high': return 'icon-[heroicons--exclamation-circle-solid]';
			case 'medium': return 'icon-[heroicons--information-circle-solid]';
			case 'low': return 'icon-[heroicons--check-circle-solid]';
			default: return 'icon-[heroicons--question-mark-circle-solid]';
		}
	}

	function formatDate(dateString: string | null) {
		if (!dateString) return 'N/A';
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="flex flex-col gap-6 px-6 py-4 w-full max-w-4xl mx-auto">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div>
			<h1 class="text-3xl font-bold tracking-tight">Alert Rules</h1>
			<p class="text-muted-foreground">Manage alert rules across your systems</p>
		</div>
		<div class="flex gap-2">
			<Button class="gap-2" onclick={() => window.location.href = '/alerts/new'}>
				<span class="icon-[heroicons--plus] w-4 h-4"></span>
				Add Rule
			</Button>
		</div>
	</div>

	<!-- Alert Rules Table -->
	<div class="rounded-lg border border-border bg-card">
		<div class="divide-y divide-border">
			{#each alertsWithSystems as alert}
				<div class="p-4 hover:bg-muted/50 transition-colors bg-gradient-to-br from-[var(--background)]/60 via-[var(--foreground)]/60 to-[var(--background)]/60 backdrop-blur-xs {alertStates[alert.id]?.isDeleting ? 'opacity-50' : ''}">
					<div class="flex items-start justify-between">
						<div class="flex items-start gap-4 flex-1">
							<!-- Status and Severity Indicator -->
							<div class="flex flex-col items-center gap-2 pt-1">
								<!-- Alert Status -->
								<div class="flex items-center gap-2">
									<div class="relative">
										<Switch
											checked={alertStates[alert.id]?.active ?? false}
											disabled={alertStates[alert.id]?.isToggling}
											onCheckedChange={() => toggleAlert(alert.id)}
											class="data-[state=checked]:bg-green-500"
										/>
										{#if alertStates[alert.id]?.isToggling}
											<div class="absolute inset-0 flex items-center justify-center">
												<div class="w-3 h-3 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
											</div>
										{/if}
									</div>
								</div>
							</div>

							<!-- Alert Info -->
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-3 mb-1">
									<button
										class="font-medium text-left hover:text-primary transition-colors cursor-pointer text-lg"
										onclick={() => window.location.href = `/alerts/edit/${alert.id}`}
									>
										{alert.name}
									</button>

									<!-- Status Badge -->
									<span class="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium rounded-full {alertStates[alert.id]?.active ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400' : 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400'}">
										<span class="w-1.5 h-1.5 rounded-full {alertStates[alert.id]?.active ? 'bg-green-500' : 'bg-gray-500'}"></span>
										{alertStates[alert.id]?.active ? 'Active' : 'Inactive'}
									</span>

									<!-- Severity Badge -->
									<span class="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium rounded-full border {getSeverityColor(alert.severity)} border-current/20 bg-current/10">
										{alert.severity}
									</span>
								</div>

								{#if alert.description}
									<p class="text-sm text-muted-foreground mb-3">{alert.description}</p>
								{:else}
									<p class="text-sm text-muted-foreground mb-3 italic">No description provided</p>
								{/if}

								<div class="flex flex-col gap-1 text-xs text-muted-foreground mb-3">
									<span>Last modified: {formatDate(alert.updated)}</span>
									<span>Expression: <code class="px-1 py-0.5 rounded bg-muted font-mono text-xs">{alert.expression}</code></span>
								</div>

								<!-- Applied Systems -->
								<div class="flex items-center gap-2">
									<span class="text-xs text-muted-foreground font-medium">Applied to:</span>
									{#if alert.systems.length === 0}
										<span class="text-xs text-muted-foreground italic">No systems assigned</span>
									{:else}
										<div class="flex items-center gap-1 flex-wrap">
											{#each alert.systems.slice(0, 3) as system}
												<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded-full bg-primary/10 text-primary border border-primary/20">
													<span class="w-1.5 h-1.5 rounded-full {system.active ? 'bg-green-500' : 'bg-red-500'}"></span>
													{system.label || system.hostname}
												</span>
											{/each}
											{#if alert.systems.length > 3}
												<Button
													variant="outline"
													size="sm"
													class="h-6 px-2 text-xs"
													onclick={() => window.location.href = `/alerts/edit/${alert.id}`}
												>
													+{alert.systems.length - 3} more
												</Button>
											{/if}
										</div>
									{/if}
								</div>
							</div>
						</div>

						<!-- Quick Actions -->
						<div class="flex items-center gap-2 ml-4">
							<!-- More Actions Dropdown -->
							<DropdownMenu.Root>
								<DropdownMenu.Trigger>
									{#snippet child({props})}
										<Button {...props } variant="ghost" size="sm" class="p-2 cursor-pointer">
											<span class="icon-[heroicons--ellipsis-horizontal] w-6 h-6"></span>
										</Button>
									{/snippet}
								</DropdownMenu.Trigger>

								<DropdownMenu.Content class="w-48">
									<DropdownMenu.Item class="cursor-pointer" onclick={() => window.location.href = `/alerts/edit/${alert.id}`}>
										<span class="icon-[heroicons--pencil] w-4 h-4 mr-2"></span>
										Edit Alert
									</DropdownMenu.Item>

									<DropdownMenu.Item class="cursor-pointer" onclick={() => viewHistory(alert.id)}>
										<span class="icon-[heroicons--clock] w-4 h-4 mr-2"></span>
										View History
									</DropdownMenu.Item>

									<DropdownMenu.Separator />

									<DropdownMenu.Item
										class="cursor-pointer text-red-600 focus:text-red-600 dark:text-red-400 dark:focus:text-red-400"
										onclick={() => deleteAlert(alert.id)}
										disabled={alertStates[alert.id]?.isDeleting}
									>
										{#if alertStates[alert.id]?.isDeleting}
											<div class="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin mr-2"></div>
											Deleting...
										{:else}
											<span class="icon-[heroicons--trash] w-4 h-4 mr-2"></span>
											Delete Alert
										{/if}
									</DropdownMenu.Item>
								</DropdownMenu.Content>
							</DropdownMenu.Root>
						</div>
					</div>
				</div>
			{/each}

			{#if alertsWithSystems.length === 0}
				<div class="py-16 text-center">
					<span class="icon-[heroicons--shield-exclamation] w-12 h-12 mx-auto mb-4 text-muted-foreground opacity-50"></span>
					<h3 class="text-lg font-medium mb-2">No alert rules found</h3>
					<p class="text-muted-foreground mb-4">Create your first alert rule to start monitoring your systems</p>
					<Button onclick={() => window.location.href = '/alerts/new'}>
						<span class="icon-[heroicons--plus] w-4 h-4 mr-2"></span>
						Create Alert Rule
					</Button>
				</div>
			{/if}
		</div>
	</div>
</div>

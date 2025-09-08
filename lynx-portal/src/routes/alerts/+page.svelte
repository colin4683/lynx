<script lang="ts">
	import { Switch } from "$lib/components/ui/switch/index.js";
	import { Button } from '$lib/components/ui/button';
	import * as Select from "$lib/components/ui/select/index.js";

	const { data } = $props();

	const alerts = $derived.by(() => {
		return data.alerts ?? [];
	});

	const systems = $derived.by(() => {
		return data.systems ?? [];
	});

	// Group systems by alert rule (since rules can be reused across systems)
	const alertsWithSystems = $derived.by(() => {
		return alerts.map(alert => ({
			...alert,
			systems: alert.alertSystems?.map(as => as.system) || []
		}));
	});

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
				<div class="p-4 hover:bg-muted/50 transition-colors bg-gradient-to-br from-[var(--background)]/60 via-[var(--foreground)]/60 to-[var(--background)]/60 backdrop-blur-xs">
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-4 flex-1">
							<!-- Severity Indicator -->
							<!-- Alert Info -->
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2 mb-1">
									<button
										class="font-medium text-left hover:text-primary transition-colors cursor-pointer"
										onclick={() => window.location.href = `/alerts/edit/${alert.id}`}
									>
										{alert.name}
									</button>
									<span class="text-xs font-medium tracking-wide {getSeverityColor(alert.severity)} min-w-fit">
										{alert.severity}
									</span>
								</div>

								{#if alert.description}
									<p class="text-sm text-muted-foreground mb-2">{alert.description}</p>
								{:else}
									<p class="text-sm text-muted-foreground mb-2 italic">No description provided</p>
								{/if}

								<div class="flex  flex-col items-start gap-0.5 text-xs text-muted-foreground mb-2">
									<span>Last modified: {formatDate(alert.updated)}</span>
									<span>Expression: <code class="px-1 py-0.5 rounded bg-muted font-mono">{alert.expression}</code></span>
								</div>

								<!-- Applied Systems -->
								<div class="flex items-center gap-2">
									<span class="text-xs text-muted-foreground">Applied to:</span>
									{#if alert.systems.length === 0}
										<span class="text-xs text-muted-foreground italic">No systems</span>
									{:else}
										<div class="flex items-center gap-1 flex-wrap">
											{#each alert.systems as system}
												<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded-full bg-primary/10 text-primary">
													<span class="w-1.5 h-1.5 rounded-full {system.active ? 'bg-green-500' : 'bg-red-500'}"></span>
													{system.label || system.hostname}
												</span>
											{/each}
											{#if alert.systems.length > 3}
												<Button
													variant="ghost"
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

						<!-- Actions -->
						<div class="flex items-center gap-2 ml-4">
							<Button variant="ghost" size="md" class="p-2 cursor-pointer">
								<span class="icon-[heroicons--ellipsis-horizontal] w-6 h-6"></span>
							</Button>
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

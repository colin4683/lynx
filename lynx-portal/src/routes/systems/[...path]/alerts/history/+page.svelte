<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { goto } from '$app/navigation';

	const { data } = $props();

	function getSeverityColor(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'text-red-500 bg-red-500/10';
			case 'high': return 'text-orange-500 bg-orange-500/10';
			case 'medium': return 'text-yellow-500 bg-yellow-500/10';
			case 'low': return 'text-green-500 bg-green-500/10';
			default: return 'text-gray-500 bg-gray-500/10';
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

	function formatDate(dateString: string) {
		const date = new Date(dateString);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

		if (diffDays === 0) {
			return `Today at ${date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })}`;
		} else if (diffDays === 1) {
			return `Yesterday at ${date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })}`;
		} else if (diffDays < 7) {
			return `${diffDays} days ago`;
		} else {
			return date.toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		}
	}
</script>

<div class="flex flex-col gap-6 w-full px-6 py-4 max-w-6xl mx-auto">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-4">
			<button
				class="icon-[ep--back] w-6 h-6 cursor-pointer text-muted-foreground hover:text-white transition-colors"
				onclick={() => goto(`/systems/${data.path}/alerts`)}
			></button>
			<div>
				<h1 class="text-2xl font-bold">Alert History</h1>
				<p class="text-muted-foreground">Complete alert history for {data.system?.label || data.system?.hostname}</p>
			</div>
		</div>
		<Button variant="outline" onclick={() => goto(`/systems/${data.path}/alerts`)}>
			<span class="icon-[heroicons--cog-6-tooth] w-4 h-4 mr-2"></span>
			Manage Alerts
		</Button>
	</div>

	<!-- Alert History -->
	<div class="space-y-4">
		{#if data.history?.length === 0}
			<div class="rounded-lg border border-dashed border-border bg-muted/20 p-12 text-center">
				<span class="icon-[heroicons--clock] w-16 h-16 mx-auto mb-4 text-muted-foreground opacity-50"></span>
				<h3 class="text-xl font-medium mb-2">No alert history</h3>
				<p class="text-muted-foreground mb-6">This system has no alert history yet. Configure alert rules to start monitoring.</p>
				<Button onclick={() => goto(`/systems/${data.path}/alerts`)}>
					<span class="icon-[heroicons--plus] w-4 h-4 mr-2"></span>
					Configure Alerts
				</Button>
			</div>
		{:else}
			<div class="rounded-lg border border-border bg-card divide-y divide-border">
				{#each data.history as historyItem}
					<div class="p-4 hover:bg-muted/20 transition-colors">
						<div class="flex items-start justify-between">
							<div class="flex items-start gap-4">
								<div class="flex-shrink-0 mt-1">
									<span class="{getSeverityIcon(historyItem.alertRule.severity)} w-5 h-5 {getSeverityColor(historyItem.alertRule.severity).split(' ')[0]}"></span>
								</div>
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-3 mb-2">
										<h3 class="font-medium">{historyItem.alertRule.name}</h3>
										<span class="inline-flex items-center gap-1 px-2 py-1 text-xs rounded-full {getSeverityColor(historyItem.alertRule.severity)}">
											<span class="{getSeverityIcon(historyItem.alertRule.severity)} w-3 h-3"></span>
											{historyItem.alertRule.severity}
										</span>
									</div>

									{#if historyItem.alertRule.description}
										<p class="text-sm text-muted-foreground mb-2">{historyItem.alertRule.description}</p>
									{/if}

									<div class="flex items-center gap-4 text-xs text-muted-foreground">
										<span>Expression: <code class="px-1 py-0.5 rounded bg-muted font-mono">{historyItem.alertRule.expression}</code></span>
									</div>
								</div>
							</div>

							<div class="flex flex-col items-end gap-1 text-sm text-muted-foreground">
								<span>{formatDate(historyItem.date)}</span>
								<Button
									variant="ghost"
									size="sm"
									onclick={() => goto(`/alerts/edit/${historyItem.alertRule.id}`)}
								>
									<span class="icon-[heroicons--pencil] w-4 h-4"></span>
								</Button>
							</div>
						</div>
					</div>
				{/each}
			</div>

			{#if data.history.length >= 50}
				<div class="text-center py-4">
					<p class="text-muted-foreground text-sm">Showing recent 50 alerts. Older alerts are automatically archived.</p>
				</div>
			{/if}
		{/if}
	</div>
</div>

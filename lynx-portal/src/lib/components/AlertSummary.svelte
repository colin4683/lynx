<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { goto } from '$app/navigation';

	interface AlertSummaryProps {
		systemId: number;
		appliedRulesCount?: number;
		recentAlertsCount?: number;
		severity?: 'none' | 'low' | 'medium' | 'high' | 'critical';
		lastAlert?: string | null;
	}

	const {
		systemId,
		appliedRulesCount = 0,
		recentAlertsCount = 0,
		severity = 'none',
		lastAlert = null
	}: AlertSummaryProps = $props();

	function getSeverityColor(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'text-red-500 bg-red-500/10 border-red-500/20';
			case 'high': return 'text-orange-500 bg-orange-500/10 border-orange-500/20';
			case 'medium': return 'text-yellow-500 bg-yellow-500/10 border-yellow-500/20';
			case 'low': return 'text-green-500 bg-green-500/10 border-green-500/20';
			default: return 'text-gray-500 bg-gray-500/5 border-gray-500/20';
		}
	}

	function getSeverityIcon(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'icon-[heroicons--exclamation-triangle-solid]';
			case 'high': return 'icon-[heroicons--exclamation-circle-solid]';
			case 'medium': return 'icon-[heroicons--information-circle-solid]';
			case 'low': return 'icon-[heroicons--check-circle-solid]';
			default: return 'icon-[heroicons--shield-check]';
		}
	}

	function getStatusText(severity: string, recentAlertsCount: number) {
		if (recentAlertsCount === 0) return 'No recent alerts';
		if (severity === 'critical') return 'Critical alerts active';
		if (severity === 'high') return 'High priority alerts';
		if (severity === 'medium') return 'Medium priority alerts';
		if (severity === 'low') return 'Low priority alerts';
		return `${recentAlertsCount} alert${recentAlertsCount > 1 ? 's' : ''}`;
	}

	function formatLastAlert(lastAlert: string | null) {
		if (!lastAlert) return null;
		const date = new Date(lastAlert);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMinutes = Math.floor(diffMs / (1000 * 60));
		const diffHours = Math.floor(diffMinutes / 60);
		const diffDays = Math.floor(diffHours / 24);

		if (diffMinutes < 1) return 'Just now';
		if (diffMinutes < 60) return `${diffMinutes}m ago`;
		if (diffHours < 24) return `${diffHours}h ago`;
		if (diffDays < 7) return `${diffDays}d ago`;
		return date.toLocaleDateString();
	}
</script>

<div class="rounded-lg border {getSeverityColor(severity)} p-4 transition-all hover:shadow-md cursor-pointer"
     onclick={() => goto(`/systems/${systemId}/alerts`)}>
	<div class="flex items-center justify-between mb-3">
		<div class="flex items-center gap-2">
			<span class="{getSeverityIcon(severity)} w-5 h-5"></span>
			<h3 class="font-medium">Alert Status</h3>
		</div>
		<Button variant="ghost" size="sm" class="p-1">
			<span class="icon-[heroicons--arrow-top-right-on-square] w-4 h-4"></span>
		</Button>
	</div>

	<div class="space-y-2">
		<div class="flex items-center justify-between text-sm">
			<span class="text-muted-foreground">Active Rules:</span>
			<span class="font-medium">{appliedRulesCount}</span>
		</div>

		<div class="flex items-center justify-between text-sm">
			<span class="text-muted-foreground">Status:</span>
			<span class="font-medium">{getStatusText(severity, recentAlertsCount)}</span>
		</div>

		{#if lastAlert}
			<div class="flex items-center justify-between text-sm">
				<span class="text-muted-foreground">Last Alert:</span>
				<span class="font-medium">{formatLastAlert(lastAlert)}</span>
			</div>
		{/if}
	</div>

	{#if appliedRulesCount === 0}
		<div class="mt-3 pt-3 border-t border-border">
			<p class="text-xs text-muted-foreground mb-2">No alert rules configured</p>
			<Button size="sm" class="w-full">
				Configure Alerts
			</Button>
		</div>
	{/if}
</div>

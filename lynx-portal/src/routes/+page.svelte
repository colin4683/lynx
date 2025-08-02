<script lang="ts">
	import ProgressBar from '$lib/ProgressBar.svelte';
	import LineChart from '$lib/components/LineChart.svelte';
	import RadialChart from '$lib/components/RadialChart.svelte';
	import MiniLineChart from '$lib/components/MiniLineChart.svelte';
	import {onDestroy} from 'svelte';

	const {data} = $props();


	function relativeDate(date: string) : string {
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

	function secondsToTime(seconds: number): string {
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		const secs = seconds % 60;
		return `${hours}h ${minutes}m ${secs}s`;
	}

	function colorFromPercent(percentage: number) : string {
		let r = Math.round(144 + (255 - 144) * (percentage / 100));
		let g = Math.round(238 + (71 - 238) * (percentage / 100));
		let b = Math.round(144 + (71 - 144) * (percentage / 100));
		return `rgb(${r},${g},${b})`;
	}
	function colorFromPercentAlpha(percentage: number) : string {
		let r = Math.round(144 + (255 - 144) * (percentage / 100));
		let g = Math.round(238 + (71 - 238) * (percentage / 100));
		let b = Math.round(144 + (71 - 144) * (percentage / 100));
		return `rgba(${r},${g},${b}, 0.4)`;
	}
	const cpuChartData = $derived.by(() => {
		return data.metrics.map(metric => ({
			time: new Date(metric.time).toLocaleTimeString('it-IT'),
			cpu: metric.cpuUsage ?? 0
		}))
	});
	const cpuChartConfig = $state({
		cpu: {
			label: "cpu",
			color: "#5c61d0"
		}
	})



</script>


<div class="w-full px-2 grid grid-cols-1 lg:grid-cols-2 gap-10">
	<div class="w-full flex flex-col justify-start items-start gap-2">
		<h1 class="text-lg font-extrabold">Welcome to the lynx dashboard</h1>
		<p class="text-center align-middle flex items-center gap-1.5">
			<span class="text-muted-foreground icon-[line-md--account] w-4 h-4"></span>
			<span class="text-sm text-muted-foreground">{data.user.email}</span>
		</p>
		<div class="flex flex-col gap-2 w-1/2">
			<h3 class="text-md">Available Systems</h3>
			<div class="flex flex-wrap gap-2 w-full">
				{#each data.systems as system}
					<div class="w-full backdrop-blur-[2px] p-3 rounded-md border border-[var(--border)] flex items-center ">
							<span class="icon-[ix--agent] mr-2 w-5 h-5 text-primary"></span>
							<div class="flex flex-col gap-1">
								<div class="flex items-center align-middle">
										<p class="text-sm font-bold">{system.label ?? system.hostname}</p>
										<p class="text-sm text-muted-foreground">@</p>
										<p class="text-sm text-muted-foreground">{system.hostname}</p>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-muted-foreground">{system.os}</span>
									</div>
							</div>
							<button class="px-2 absolute right-2 py-1 text-xs rounded bg-primary text-white hover:bg-primary/90" onclick={() => {
								window.location.href = `/systems/${system.id}`;
							}}>View</button>
					</div>
				{/each}
			</div>
		</div>
	</div>

	<div class="w-full flex flex-col gap-1 bg-foreground px-4 py-2 shadow-xl border border-border rounded-lg">
		<h2 class="text-lg font-semibold">Recent alerts</h2>
		{#if data.alerts.length > 0}
			<div class="flex flex-col gap-2">
				{#each data.alerts as alert, i}
					<div class="w-full bg-background p-3 rounded-md border border-[var(--border)] flex items-center justify-between">
						<div class="flex items-center gap-2">
							<span class="icon-[line-md--alert] w-5 h-5 text-red-500"></span>
							<p class="text-sm font-bold">{alert.system?.label ?? alert.system?.hostname ?? 'Unknown Agent'}</p>
							<p class="text-sm">{alert.alertRule.name}</p>
						</div>
						<div class="flex items-center gap-2">
							<p class="text-sm text-muted-foreground">{relativeDate(alert.date)}</p>
							<button class="px-2 py-1 text-xs rounded bg-red-100 text-red-700 hover:bg-red-200" onclick={() => {
								data.alerts.splice(i, 1);

							}}>Dismiss</button>
						</div>
					</div>
				{/each}
			</div>
		{:else}
			<p class="text-sm text-muted-foreground">No recent alerts</p>
		{/if}
	</div>
</div>
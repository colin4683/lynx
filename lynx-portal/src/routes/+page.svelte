<script lang="ts">
	import ProgressBar from '$lib/ProgressBar.svelte';
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

</script>


<div class="w-full px-2 flex flex-col gap-10">
	<h1 class="text-lg font-bold">Welcome to the lynx dashboard</h1>


	<div class="w-full bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
		<p class="text-sm font-bold font-mono">Lynx Hub - system information</p>
		{#if data.hub}
			<div class="flex w-full items-center align-middle gap-4">
				<div class="flex items-center gap-1">
					<span class="icon-[solar--home-wifi-bold] w-5 h-5w"></span>
					<span class="text-sm font-mono">{data.hub.hostname}</span>
				</div>
				<div class="w-0.5 h-7 bg-[var(--border)]"></div>
				<div class="flex items-center gap-1">
					<span class="icon-[solar--monitor-linear] w-5 h-5"></span>
					<span class="text-sm font-mono">{data.hub.os}</span>
				</div>
				<div class="w-0.5 h-7 bg-[var(--border)]"></div>
				<div class="flex items-center gap-1">
					<span class="icon-[solar--clock-circle-broken] w-5 h-5"></span>
					<span class="text-sm font-mono">{secondsToTime(data.hub.uptime ?? 0)}</span>
				</div>
				<div class="w-0.5 h-7 bg-[var(--border)]"></div>
				<div class="flex items-center gap-1">
					<span class="icon-[solar--cpu-bolt-linear] w-5 h-5"></span>
					<span class="text-sm font-mono">{data.hub.cpu}</span>
				</div>
			</div>
			<div class="w-full flex justify-between  align-middle items-center">
				<div class="max-w-sm w-full">
					<p class="text-sm font-mono mt-2">CPU Usage: {((data.hub.cpuUsage ?? 0) * 100).toFixed(2)}%</p>
					<ProgressBar min={0} max={1} value={data.hub.cpuUsage ?? 0} />
				</div>
				<div class="max-w-sm w-full">
					{#if data.hub.memoryTotal && data.hub.memoryUsed}
						<p class="text-sm font-mono mt-2">Memory Usage: {((data.hub.memoryUsed / data.hub.memoryTotal)* 100).toFixed(2)}%</p>
						<ProgressBar min={0} max={1} value={(data.hub.memoryUsed / data.hub.memoryTotal)} />
					{/if}
				</div>
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-2">
		<h3 class="text-md">Available Systems</h3>
		<table class="w-full bg-[var(--foreground)]">
			<thead>
			<tr>
				<th>System Name</th>
				<th>System ID</th>
				<th>System Type</th>
			</tr>
			</thead>
			<tbody>
			{#each data.systems as system}
				<tr>
					<td>{system.address}</td>
					<td>{system.id}</td>
					<td>{relativeDate(system.lastSeen ?? "")}</td>
				</tr>
			{/each}
			</tbody>
		</table>
	</div>
</div>
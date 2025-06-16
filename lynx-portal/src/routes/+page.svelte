<script lang="ts">
	import ProgressBar from '$lib/ProgressBar.svelte';
	import LineChart from '$lib/components/LineChart.svelte';
	import RadialChart from '$lib/components/RadialChart.svelte';
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
	const cpuChartData = $derived.by(() => {
		return [{
			key: "cpu",
			color: "#4f46e5",
			data: [{
				cpu:data.hub?.cpuUsage ?? 0
			}]
		}]
	});

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
</script>


<div class="w-full px-2 flex flex-col gap-10">
	<h1 class="text-lg font-extrabold">Welcome to the lynx dashboard</h1>


	<div class="w-full bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
		<p class="text-xl font-bold font-mono">Lynx Hub - system information</p>
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
			</div>
			<div class="w-full flex justify-between gap-2  align-middle items-center">
				<div
					class={`max-w-sm w-full bg-[var(--background)] relative border  flex flex-col items-start  py-1 pl-2.5 rounded-xl shadow-md`}
					style={`border: 1px solid ${colorFromPercent(data.hub.cpuUsage ?? 0.0)}`}
				>
					<p class="text-lg font-bold flex items-center align-middle gap-1">
						<span class="icon-[solar--cpu-bolt-linear] w-5 h-5"></span>
						CPU
					</p>
					<p class="text-xs text-muted-foreground">{data.hub.cpu}</p>
					<p class="absolute right-3 bottom-1/4">{(data.hub.cpuUsage ?? 0.0).toFixed(2)}%</p>
				</div>
				{#if data.hub.memoryTotal && data.hub.memoryUsed}
					<div
						class={`max-w-sm w-full bg-[var(--background)] relative border  flex flex-col items-start  py-1 pl-2.5 rounded-xl shadow-md`}
						style={`border: 1px solid ${colorFromPercent(data.hub.memoryUsed / data.hub.memoryTotal * 100)}`}
					>
						<p class="text-lg font-bold flex items-center align-middle gap-1">
							<span class="icon-[ri--ram-line] w-5 h-5"></span>
							Memory
						</p>
						<p class="text-xs text-muted-foreground">Total: {(data.hub.memoryTotal / 1024 / 1024).toFixed(0)}gb</p>
						<p class="absolute right-3 bottom-1/4">{(data.hub.memoryUsed / data.hub.memoryTotal * 100).toFixed(2)}%</p>
					</div>
				{/if}
				{#if data.hub.disks[0].used && data.hub.disks[0].space}
					<div
						class={`max-w-sm w-full bg-[var(--background)] z-[5] relative border  flex flex-col items-start  py-1 pl-2.5 rounded-lg shadow-md`}
						style={`border: 1px solid ${colorFromPercent(data.hub.disks[0].used / data.hub.disks[0].space * 100)}`}
					>
						<p class="text-lg font-bold flex items-center align-middle gap-1">
							<span class="icon-[ri--ram-line] w-5 h-5"></span>
							Storage
						</p>
						<p class="text-xs text-muted-foreground">Capacity: {(data.hub.disks[0].space / 1024 / 1024).toFixed(0)}gb</p>
						<p class="absolute right-3 bottom-1/4">{(data.hub.disks[0].used / data.hub.disks[0].space * 100).toFixed(2)}%</p>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-2">
		<h3 class="text-md">Available Systems</h3>
		<table class="w-full bg-[var(--foreground)]">
			<thead>
			<tr>
				<th>System</th>
				<th>
					<span class="icon-[solar--cpu-bolt-linear] w-5 h-5">CPU</span>
					CPU
				</th>
				<th>
					<span class="icon-[ri--ram-line] w-5 h-5">MEMORY</span>
					Memory
				</th>
				<th>
					<span class="icon-[mdi--hdd] w-5 h-5">DISK</span>
					Disk
				</th>
				<th>
					<span class="icon-[fluent--network-check-24-regular] w-5 h-5">NETWORK</span>
					Net
				</th>
				<th>
					<span class="icon-[lucide--clock] w-5 h-5">UPDATED</span>
					Updated
				</th>
			</tr>
			</thead>
			<tbody>
			{#each data.systems as system}
				<tr>
					<td>
						<div class="flex items-center justify-start gap-2 group  w-full  cursor-pointer" onclick={() => window.open(`/systems/${system.id}`, '_self')}>
							<span class={`w-2 h-2 rounded-full ${system.active ? 'bg-green-300' : 'bg-red-400'}`}></span>
							<p class="font-bold group-hover:text-[var(--primary)]">{system.label}</p>
							<span class="icon-[cuida--open-in-new-tab-outline] w-3.5 h-3.5 group-hover:text-[var(--primary)]">open</span>
						</div>
					</td>
					<td>
						{#if system.cpuUsage}
							<div class="flex align-middle items-center gap-2 px-3">
								<p class="text-sm">{(system.cpuUsage).toFixed(2)}%</p>
								<ProgressBar min={0} max={1} value={system.cpuUsage / 100} type="cpu" />
							</div>
						{/if}
					</td>
					<td>
						{#if system.memoryTotal && system.memoryUsed}
							<div class="flex align-middle items-center gap-2 px-3">
								<p class="text-sm">{((system.memoryUsed / system.memoryTotal)* 100).toFixed(2)}%</p>
								<ProgressBar min={0} max={1} value={(system.memoryUsed / system.memoryTotal)} type="memory" />
							</div>
						{/if}
					</td>
					<td>
						{#if system.disks[0].used && system.disks[0].space && system.disks[0].time}
							<div class="flex align-middle items-center gap-2 px-3">
								<p class="text-sm">{((system.disks[0].used / system.disks[0].space)* 100).toFixed(2)}%</p>
								<ProgressBar min={0} max={1} value={system.disks[0].used / system.disks[0].space} type="disk" />
							</div>
						{/if}
					</td>
					<td>
						<p class="text-sm text-left">
							{system.metrics[0].netIn ? (system.metrics[0].netIn / 1024).toFixed(2) : 0}mb/s
						</p>
					</td>
					<td>
						<p class="text-sm text-left">
							{relativeDate(system.metrics[0].time)}
						</p>
					</td>
				</tr>
			{/each}
			</tbody>
		</table>
	</div>
</div>
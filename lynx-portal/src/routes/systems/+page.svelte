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

	/* <span class="icon-[fluent--alert-on-24-filled]"></span> */

</script>


<div class="flex flex-col gap-2 w-full px-20">
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
			<th class="border-r-0" style="border-right: 0">
				<span class="icon-[lucide--clock] w-5 h-5">UPDATED</span>
				Updated
			</th>
			<th class="border-l-0" style="border-left: 0">
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
				<td class="flex items-center justify-end pr-2 gap-2">
					<span class="icon-[fluent--alert-32-regular] w-5 h-5 cursor-pointer"></span>
					<span class="icon-[tabler--dots] w-5 h-5 cursor-pointer"></span>
				</td>
			</tr>
		{/each}
		</tbody>
	</table>
</div>

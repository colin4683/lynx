<script lang="ts">
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

</script>


<div class="w-full">
	<h2>Available Systems</h2>
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
				<td>{system.lastSeen}</td>
			</tr>
		{/each}
		</tbody>
	</table>

	<h2>Metrics</h2>
	<table class="w-full bg-[var(--foreground)]">
		<thead>
		<tr>
			<th>System</th>
			<th>Date</th>
			<th>CPU Usage</th>
			<th>Memory Used</th>
		</tr>
		</thead>
		<tbody>
		{#each data.metrics as metric}
			<tr>
				<td>{metric.systemId}</td>
				<td>{relativeDate(metric.time)}</td>
				<td>{metric.cpuTemp}</td>
				<td>{metric.memoryUsedKb ? (metric.memoryUsedKb / 1024 / 1024 / 1024).toFixed(2) : 0}gb</td>
			</tr>
		{/each}
		</tbody>
	</table>
</div>
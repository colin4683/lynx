<script lang="ts">

	import ProgressBar from '$lib/ProgressBar.svelte';

	const {data} = $props();

</script>


<div class="flex flex-col gap-2 w-full px-20">
	<div class="flex items-center align-middle gap-2">
		<span class="icon-[ep--back] w-5 h-5 cursor-pointer text-muted-foreground hover:text-white transition-colors" onclick={() => window.location.href = `/systems/${data.path}`}></span>
		<h3 class="text-md">Alert History</h3>
	</div>
	<table class="w-full bg-[var(--foreground)]">
		<thead>
		<tr>
			<th>System</th>
			<th>
				Alert
			</th>
			<th class="border-r-0" style="border-right: 0">
				<span class="icon-[lucide--clock] w-5 h-5">UPDATED</span>
				Date
			</th>
			<th class="border-l-0" style="border-left: 0">
			</th>
		</tr>
		</thead>
		<tbody>
		{#each data.history as alert}
			<tr>
				<td>
					{alert.system.label}
				</td>
				<td>
					{alert.alertRule.name}
				</td>
				<td>
					{alert.date}
				</td>
			</tr>
		{/each}
		</tbody>
	</table>
</div>

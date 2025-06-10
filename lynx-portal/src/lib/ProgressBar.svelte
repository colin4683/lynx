<script lang="ts">
	let {min, max, value}: {
		min: number;
		max: number;
		value: number;
	} = $props();

	let percentage = $derived.by(() => {
		if (max === 0) return 0;
		return Math.min(Math.max((value - min) / (max - min), 0), 1) * 100;
	})
</script>

<div class="w-full h-1 bg-[var(--border)] rounded-full overflow-hidden">
	<div class="h-full bg-[var(--primary)] transition-all duration-300" style="width: {percentage}%"></div>
</div>
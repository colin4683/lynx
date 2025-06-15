<script lang="ts">

	let {min, max, value, type}: {
		min: number;
		max: number;
		value: number;
		type: 'cpu' | 'memory' | 'disk' | 'network';
	} = $props();

	let percentage = $derived.by(() => {
		if (max === 0) return 0;
		return Math.min(Math.max((value - min) / (max - min), 0), 1) * 100;
	})

	let bg_color = $derived.by(() => {
		// convert percent to shade between pastel green and pastel red
		let inverse_percentage = 100 - percentage;
		let r = Math.round(144 + (255 - 144) * (percentage / 100));
		let g = Math.round(238 + (71 - 238) * (percentage / 100));
		let b = Math.round(144 + (71 - 144) * (percentage / 100));
		return `rgb(${r},${g},${b})`;
	})


</script>

<div class="w-full h-2.5 bg-[var(--border)] rounded-full overflow-hidden">
	<div class={`h-full transition-all duration-300`} style="width: {percentage}%; background: {bg_color}"></div>
</div>
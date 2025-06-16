<script lang="ts">
	import * as Chart from "$lib/components/ui/chart/index.js";
	import { ArcChart, Text } from "layerchart";

	const { config, data, x, y } : {
		config: Chart.ChartConfig,
		data: any,
		x: string,
		y: string
	} = $props();
</script>

<Chart.Container config={config} class="h-[300px] w-full">
	<ArcChart
		label="cpu"
		value="cpu"
		outerRadius={-90}
		innerRadius={-8}
		padding={3}
		range={[90, -270]}
		maxValue={100}
		series={data}
		cornerRadius={80}
		props={{
          arc: { track: { fill: "var(--muted)" }, motion: "tween" },
          tooltip: { context: { hideDelay: 350 } },
        }}
		tooltip={false}
	>
		{#snippet belowMarks()}
			<circle cx="0" cy="0" r="60" class="fill-background" />
		{/snippet}

		{#snippet aboveMarks()}
			<Text
				value={String(data[0].data[0].cpu.toFixed(2))}
				textAnchor="middle"
				verticalAnchor="middle"
				class="fill-muted-foreground text-sm! font-bold"
				dy={0}
			/>
		{/snippet}
	</ArcChart>
</Chart.Container>

<script lang="ts">
	import * as Chart from "$lib/components/ui/chart/index";
	import {LineChart} from "layerchart";
	import { scaleBand, scaleUtc } from 'd3-scale';
	import { curveNatural, curveLinear, curveMonotoneX, curveBasis } from "d3-shape";
	import { cubicInOut } from "svelte/easing";

	const {config, data, x, y, stack, format} : {
		config: Chart.ChartConfig,
		data: any,
		x: string,
		y: string,
		stack?: "overlap" | "stack" | "stackExpand" | "stackDiverging",
		format?: (d: any) => string
	} = $props();

	const series = $derived.by(() => {
		return Object.keys(config).map(obj => {
			return {
				key: obj,
				label: config[obj].label,
				color: config[obj].color
			}
		})
	})

</script>

<Chart.Container config={config} class="w-[124px] h-[25px] bg-[var(--foreground)] border border-border/40">
	<LineChart
		data={data}
		xScale={scaleBand().padding(0.25)}
		x={x}
		axis={false}
		grid={false}
		seriesLayout={stack ?? "stack"}
		series={series}>
		{#snippet tooltip()}
			<Chart.Tooltip
			/>
		{/snippet}
	</LineChart>
</Chart.Container>

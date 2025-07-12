<script lang="ts">
	import * as Chart from "$lib/components/ui/chart/index";
	import {AreaChart, Area, LinearGradient} from "layerchart";
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

<Chart.Container config={config} class="h-[300px] w-full">
	<AreaChart
		data={data}
		xScale={scaleBand().padding(0)}
		x={x}
		yNice
		padding={{
			top: 50,
			right: 10,
			bottom: 0,
			left: 50
		}}
		legend
		props={{
			area: {
				curve: curveMonotoneX,
				"fill-opacity": 0.3,
				line: {class: "stroke-1"},
				motion: "tween",
 			},
			xAxis: {
				format: (d) => d.slice(0, 5),
			},
			yAxis: { format: format ?? ((d) => d.toString()) },
		}}
		seriesLayout={stack ?? "stack"}
		series={series}>


		{#snippet tooltip()}
			<Chart.Tooltip
			/>
		{/snippet}

		{#snippet marks({ series, getAreaProps })}
			{#each series as s, i (s.key)}
				<LinearGradient
					stops={[
						s.color ?? "",
						"color-mix(in lch, " + s.color + " 10%, transparent)",
					]}
					vertical
				>
					{#snippet children({ gradient })}
						<Area {...getAreaProps(s, i)} fill={gradient} />
					{/snippet}
				</LinearGradient>
			{/each}
		{/snippet}
	</AreaChart>
</Chart.Container>

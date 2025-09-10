<script lang="ts">
	import * as Chart from "$lib/components/ui/chart/index";
	import {AreaChart, Area, LinearGradient} from "layerchart";
	import { scaleBand, scaleUtc } from 'd3-scale';
	import { curveMonotoneX } from "d3-shape";
	import { onMount } from "svelte";
	import { cubicInOut } from "svelte/easing";

	const {config, data, x, y, stack, format, onEvent} : {
		config: Chart.ChartConfig,
		data: any,
		x: string,
		y: string,
		stack?: "overlap" | "stack" | "stackExpand" | "stackDiverging",
		format?: (d: any) => string,
		onEvent: (start: number, end: number) => void | undefined
	} = $props();

	const series = $derived.by(() => {
		return Object.keys(config).map(obj => ({
			key: obj,
			label: config[obj].label,
			color: config[obj].color
		}))
	});

	let isSelecting = $state(false);
	let selectionStart: number | null = $state(null);
	let selectionEnd: number | null = $state(null);
	let chartRef: any | null = null;
	let zoomedData = $derived.by(() => {
		return data;
	});

	let isZoomed = $state(false);
	function getXValueFromClientX(clientX: number) {
		if (!chartRef) return 0;
		const rect = chartRef.getBoundingClientRect();
		const relX = clientX - rect.left;
		const width = rect.width;
		const idx = Math.round((relX / width) * (data.length - 1));
		return Math.max(0, Math.min(data.length - 1, idx));
	}

	function onMouseDown(e: MouseEvent) {
		isSelecting = true;
		selectionStart = getXValueFromClientX(e.clientX);
		selectionEnd = null;
		console.log("Selection started at:", selectionStart);
	}

	function onMouseMove(e: MouseEvent) {
		if (!isSelecting) return;
		selectionEnd = getXValueFromClientX(e.clientX);
	}

	function onMouseUp(e: MouseEvent) {
		if (!isSelecting || selectionStart === null) return;
		selectionEnd = getXValueFromClientX(e.clientX);
		isSelecting = false;
		if (selectionStart !== null && selectionEnd !== null && selectionStart !== selectionEnd) {
			// flip if necessary
			if (selectionStart > selectionEnd) {
				[selectionStart, selectionEnd] = [selectionEnd, selectionStart];
			}
			const [start, end] = [selectionStart, selectionEnd].sort((a, b) => a - b);
			const startValue = data[start][x];
			const endValue = data[end][x];
			if (onEvent) {
				onEvent(startValue, endValue);
			}
			isZoomed = true;
		}
	}

	function resetZoom() {
		zoomedData = data;
		selectionStart = null;
		selectionEnd = null;
		isZoomed = false;
	}

</script>
<div
	aria-roledescription="tooltip"
	bind:this={chartRef}
	class="h-[300px] w-full overflow-clip relative"
	onmousedown={onMouseDown}
	onmousemove={onMouseMove}
	onmouseup={onMouseUp}
	onmouseleave={() => isSelecting = false}
>
	<Chart.Container
		config={config}
		class="h-[300px] w-full overflow-clip relative"

	>
			<AreaChart
				data={zoomedData}
				xScale={scaleBand().padding(0)}
				x={x}
				y={y}
				yNice
				padding={{
			 top: 50,
			 right: 10,
			 bottom: 20,
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
				format: (d) => d ? d.slice(0, 5) : "",
			 },
			 yAxis: { format: format ?? ((d) => d.toString()) },
			}}
				seriesLayout={stack ?? "overlap"}
				series={series}>
				{#snippet tooltip()}
					<Chart.Tooltip indicator="dot"
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
			{#if isSelecting && selectionStart !== null && selectionEnd !== null}
				<div
					class="absolute flex justify-center items-center  top-0 left-0  h-full bg-background/30 border border-primary/60 pointer-events-none"
					style="z-index:10;
					 left: {Math.min(selectionStart, selectionEnd) / (data.length - 1) * 100}%;
					 width: {Math.abs(selectionEnd - selectionStart) / (data.length - 1) * 100}%;
				 "
				>
					<p class="text-xs font-mono">
						Selected: {data[selectionStart][x]} to {data[selectionEnd][x]}
					</p>
				</div>
			{/if}
			{#if isZoomed}
				<button class="absolute top-2 right-2 z-20 bg-background/80 border cursor-pointer border-border px-2 py-1 rounded text-xs" onclick={resetZoom}>
					Reset Zoom
				</button>
			{/if}
		</Chart.Container>
</div>
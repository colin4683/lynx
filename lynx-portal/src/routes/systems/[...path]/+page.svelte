<script lang="ts">
	import ProgressBar from '$lib/ProgressBar.svelte';
	import LineChart from '$lib/components/LineChart.svelte';
	import CPUChart from '$lib/components/CPUChart.svelte';
	import { goto, invalidate, invalidateAll } from '$app/navigation';
	import { page } from '$app/stores';
	import * as Select from '$lib/components/ui/select';

	const { data } = $props();
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

	let range = $state($page.url.searchParams.get("range") ?? "30 minutes");

	let interval = $derived.by(() => {
		switch (range) {
			case "5 minutes":
				return "1";
			case "30 minutes":
				return "5";
			case "1 hour":
				return "5";
			case "3 hours":
				return "15";
			case "12 hours":
				return "60";
			case "24 hours":
				return "120";
			default:
				return "1";
		}
	})

	const cpuChartData = $derived.by(() => {
		return data.metrics.map((metric : any) => ({
			time: new Date(metric.time_slot).toLocaleTimeString('it-IT'),
			cpu: metric.cpu_total ? metric.cpu_total : 0
		}));
	});
	const cpuChartConfig = $derived.by(() => {

		let percentage = data.metrics[data.metrics.length - 1]?.cpu_total as number || 0;
		let r = Math.round(144 + (255 - 144) * (percentage / 100));
		let g = Math.round(238 + (71 - 238) * (percentage / 100));
		let b = Math.round(144 + (71 - 144) * (percentage / 100));
		// make array of colors going down from current percentage to 0

		return {
			cpu: {
				label: "cpu",
				color: `rgb(${r},${g},${b})`,
			}
		}
	})

	const memoryChartData = $derived.by(() => {
		return data.metrics.map((metric: any) => ({
			time: new Date(metric.time_slot).toLocaleTimeString('it-IT'),
			used: metric.used_total ? metric.used_total / 1024 / 1024 : 0,
			total: metric.memory_total_kb / 1024 / 1024,
		}))
	});
	const memoryChartConfig = $state({
		used: {
			label: "used",
			color: "#e21f88"
		},
		total: {
			label: "total",
			color: "#720f44"
		}
	})

	const loadChartData = $derived.by(() => {
		return data.metrics.map((metric: any) => ({
			time: new Date(metric.time_slot).toLocaleTimeString('it-IT'),
			load_one: metric.one_total ? metric.one_total : 0,
			load_five: metric.five_total ? metric.five_total : 0,
			load_fifteen: metric.fifteen_total ? metric.fifteen_total : 0
		}))
	});
	const loadChartConfig = $state({
		load_one: {
			label: "load_one",
			color: "#7be2f5"
		},
		load_five: {
			label: "load_five",
			color: "#ac7bf5"
		},
		load_fifteen: {
			label: "load_fifteen",
			color: "#7bf593"
		}
	})

	const networkChartData = $derived.by(() => {
		return data.metrics.map((metric : any) => ({
			time: new Date(metric.time_slot).toLocaleTimeString('it-IT'),
			net_in: metric.in_total ? metric.in_total / 1024 : 0,
			net_out: metric.out_total ? metric.out_total / 1024 : 0
		}))
	})
	const networkChartConfig = $state({
		net_in: {
			label: "net_in",
			color: "#a0ff54"
		},
		net_out: {
			label: "net_out",
			color: "#ff5454"
		}
	})

	const diskChartData = $derived.by(() => {
		return data.disks.map((disk: any) => ({
			time: new Date(disk.time_slot).toLocaleTimeString('it-IT'),
			read: disk.read_total ? disk.read_total / 1024 / 1024 : 0,
			write: disk.write_total ? disk.write_total / 1024 / 1024 : 0,
		}))
	})
	const diskChartConfig = $state({
		read: {
			label: "read",
			color: "#3e28fd"
		},
		write: {
			label: "write",
			color: "#fd7a28"
		}
	})

	const storageChartData = $derived.by(() => {
		return data.disks.map((disk: any) => ({
			time: new Date(disk.time_slot).toLocaleTimeString('it-IT'),
			used: disk.used_total ? disk.used_total : 0,
			space: disk.total,
		}))
	})
	const storageChartConfig = $state({
		used: {
			label: "used",
			color: "#5e40ec"
		},
		space: {
			label: "space",
			color: "#24195d"
		}
	})

	const temperatureData = $derived.by(() => {
		return data.metrics.map((metric: any) => {
			let component_temp_array = metric.component_temperatures;
			// combine all components into a single object for each time slot
			return {
				time: new Date(metric.time_slot).toLocaleTimeString('it-IT'),
				...Object.fromEntries(
					Object.entries(component_temp_array).map(([key, value]) => [(value as any).label, (value as any).avg_temperature])
				)
			}
		})
	})

	const temperatureConfig = $state({
		Composite: {
			label: "Composite",
			color: "#ffcc00"
		},
		'iwlwifi_1 temp1': {
			label: "iwlwifi_1 temp1",
			color: "#ff6600"
		},
		Tccd1: {
			label: "Tccd1",
			color: "#ff3300"
		},
		Tctl: {
			label: "Tctl",
			color: "#ff0000"
		},
	});

/*	const temperatureConfig = $derived.by(() => {
		let config: any = {};
		if (temperatureData.length > 0) {
			let lastMetric = temperatureData[temperatureData.length - 1];
			Object.keys(lastMetric[0]).forEach((key: string) => {
				if (key !== 'time') {
					config[key] = {
						label: key,
						color: `hsl(${Math.random() * 360}, 70%, 50%)`
					}
				}
			});
		}
		return config;
	});*/


	console.log("Temperature Data:", temperatureData);


</script>

<div class="w-full bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
	<div class="w-full flex items-center align-middle justify-between">
		<p class="text-xl font-bold">
			{data.system.label}
			<span class={`w-3.5 h-3.5 inline-block rounded-full border animate-pulse ${data.system.active ? 'bg-green-300/60 border-green-400' : 'bg-red-400/60 border-red-400'}`}></span>
		</p>
		<div>
			<Select.Root type="single" bind:value={range}  onValueChange={(val) => {
			$page.url.searchParams.set("range", range);
			$page.url.searchParams.set("interval", interval);
			goto($page.url.pathname + "?" + $page.url.searchParams.toString(), {
				invalidate: ['app:systems']
			 });
		}}>
				<Select.Trigger class="w-[180px] flex items-center align-middle gap-0">
					<span class="flex items-center gap-2">
						<span class="icon-[hugeicons--date-time] w-4 h-4"></span>
						<span class="text-sm">{range}</span>
					</span>
				</Select.Trigger>
				<Select.Content class="bg-[var(--background)] rounded-md border border-[var(--border)]">
					<Select.Item value="5 minutes">5 minutes</Select.Item>
					<Select.Item value="30 minutes">30 minutes</Select.Item>
					<Select.Item value="1 hour">1 hour</Select.Item>
					<Select.Item value="3 hours">3 hours</Select.Item>
					<Select.Item value="12 hours">12 hours</Select.Item>
					<Select.Item value="24 hours">24 hours</Select.Item>
				</Select.Content>
			</Select.Root>
		</div>
	</div>
	<div class="flex w-full items-center align-middle gap-4">
		<div class="flex items-center gap-1">
			<span class="icon-[solar--home-wifi-bold] w-5 h-5w"></span>
			<span class="text-sm font-mono">{data.system.hostname}</span>
		</div>
		<div class="w-0.5 h-7 bg-[var(--border)]"></div>
		<div class="flex items-center gap-1">
			<span class="icon-[solar--monitor-linear] w-5 h-5"></span>
			<span class="text-sm font-mono">{data.system.os}</span>
		</div>
		<div class="w-0.5 h-7 bg-[var(--border)]"></div>
		<div class="flex items-center gap-1">
			<span class="icon-[solar--clock-circle-broken] w-5 h-5"></span>
			<span class="text-sm font-mono">{secondsToTime(data.system.uptime ?? 0)}</span>
		</div>
		<div class="w-0.5 h-7 bg-[var(--border)]"></div>
		<div class="flex items-center gap-1">
			<span class="icon-[solar--cpu-bolt-linear] w-5 h-5"></span>
			<span class="text-sm font-mono">{data.system.cpu}</span>
		</div>
	</div>

</div>


<div class="w-full grid grid-cols-1 lg:grid-cols-2 gap-3">
	<div class="w-full row-span-2 bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
		<h1 class="text-lg font-extrabold">CPU Usage</h1>
		<p class="text-xs text-muted-foreground -mt-3">Percentage of CPU usage over the last {range}.</p>
		<CPUChart
			config={cpuChartConfig}
			data={cpuChartData}
			x="time"
			y="cpu"
			format={(d) => `${d}%`}
		/>
	</div>

	<div class="w-full row-span-2 bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
		<h1 class="text-lg font-extrabold">Memory Usage</h1>
		<p class="text-xs text-muted-foreground -mt-3">Memory usage in gigabytes over the last {range}.</p>
		<LineChart
			config={memoryChartConfig}
			data={memoryChartData}
			x="time"
			y="total"
			stack="overlap"
			format={(d) => `${d.toFixed(2)}gb`}
		/>
	</div>

	<div class="row-span-2 bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
		<h1 class="text-lg font-extrabold">Load</h1>
		<p class="text-xs text-muted-foreground -mt-3">Number of processes using or waiting for CPU resources over the last one,five,and fifteen minutes, respectively.</p>
		<LineChart
			config={loadChartConfig}
			data={loadChartData}
			x="time"
			y="load_one"
			format={(d) => `${d.toFixed(2)}`}
		/>
	</div>

	<div class="row-span-2 bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]">
		<h1 class="text-lg font-extrabold">Network Usage</h1>
		<p class="text-xs text-muted-foreground -mt-3">Network traffic in megabytes per second over the last {range}.</p>
		<LineChart
			config={networkChartConfig}
			data={networkChartData}
			x="time"
			y="net_in"
			format={(d) => `${d.toFixed(2)}mb/s`}
		/>
	</div>

	<div class={`row-span-2 bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]`}>
		<h1 class="text-lg font-extrabold">Disk I/O</h1>
		<p class="text-xs text-muted-foreground -mt-3 ">Disk read and write speeds in megabytes per second over the last {range}.</p>
		<LineChart
			config={diskChartConfig}
			data={diskChartData}
			x="time"
			y="read"
			format={(d) => `${d.toFixed(2)}mb/s`}
		/>
	</div>

	<div class={`row-span-2 bg-[var(--background-alt)] rounded-md p-5 flex flex-col gap-3 border border-[var(--border)]`}>
		<h1 class="text-lg font-extrabold">Component Temperatures</h1>
		<p class="text-xs text-muted-foreground -mt-3 ">Component temperatures over the last {range}.</p>
		<LineChart
			config={temperatureConfig}
			data={temperatureData}
			x="time"
			y=""
			stack="overlap"
			format={(d) => `${d.toFixed(2)}°C`}
		/>
	</div>
</div>
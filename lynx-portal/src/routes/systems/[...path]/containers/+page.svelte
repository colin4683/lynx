<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { toast } from 'svelte-sonner';

	const {data} = $props();

	let containers = $derived.by(() => data.containers || []);
	let executing = $state(false);
	let search = $state('');
	let refreshInterval: NodeJS.Timeout;

	let filteredContainers = $derived.by(() => {
		if (!search) return containers;
		return containers.filter(container =>
			container.name.toLowerCase().includes(search.toLowerCase()) ||
			container.dockerId.toLowerCase().includes(search.toLowerCase())
		);
	});

	function formatDate(dateString: string) {
		const date = new Date(dateString);
		return date.toLocaleString();
	}

	function getStateStyle(state: string) {
		switch (state?.toLowerCase()) {
			case 'running':
				return 'text-green-400';
			case 'exited':
				return 'text-red-400';
			case 'paused':
				return 'text-yellow-400';
			default:
				return 'text-gray-400';
		}
	}

	async function handleContainerAction(containerId: string, action: 'start' | 'stop' | 'restart') {
		executing = true;
		let success = false;

		try {
			await new Promise((resolve, reject) => {
				socket.onopen = () => {
					socket.send(JSON.stringify({
						type: `${action}service`,
						origin: 'docker',
						service_name: containerId,
					}));
				};

				socket.onerror = (error) => {
					console.error('WebSocket error:', error);
					reject(error);
				};

				socket.onmessage = (event) => {
					const data = event.data;
					if ((data as string).toLowerCase().includes('failed')) {
						reject(new Error(data));
					} else {
						success = true;
						resolve(data);
					}
				};

				socket.onclose = () => {
					if (!success) {
						reject(new Error('WebSocket closed before operation completed'));
					} else {
						resolve(null);
					}
				};
			});

			if (success) {
				const updatePromise = fetch(`/api/containers/${containerId}/${action}`, {
					method: 'POST'
				}).then(async (response) => {
					if (!response.ok) {
						throw new Error(`HTTP error! status: ${response.status}`);
					}
					const data = await response.json();
					return data;
				});

				toast.promise(updatePromise, {
					loading: `${action.charAt(0).toUpperCase() + action.slice(1)}ing container...`,
					success: `Container ${action}ed successfully`,
					error: `Failed to ${action} container`
				});

				await updatePromise;
			}
		} catch (error: unknown) {
			console.error(`Error performing ${action} action:`, error);
			const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
			toast.error(`Failed to ${action} container: ${errorMessage}`);
		} finally {
			executing = false;
		}
	}

	onMount(() => {
		// Initial refresh
	});

	onDestroy(() => {
		if (refreshInterval) {
			clearInterval(refreshInterval);
		}
	});
</script>

<style>
    .glass-bg {
        background: rgba(30, 30, 30, 0.4);
        border: 1px solid rgba(255,255,255,0.08);
        box-shadow: 0 4px 32px 0 rgba(0,0,0,0.25);
        backdrop-filter: blur(1px);
    }
</style>

<div class="min-h-screen w-full flex flex-col items-center justify-start py-12">
	<div class="flex items-center justify-between w-full max-w-4xl mb-6 px-6">
		<button class="flex items-center px-3 py-1 glass-bg rounded hover:bg-white/10 text-gray-200 border border-white/10 cursor-pointer" onclick={() => window.history.back()}>
			<span class="icon-[line-md--arrow-left] w-5 h-5"></span>
			Back
		</button>
	</div>

	<div class="w-full max-w-4xl glass-bg rounded-xl p-8 shadow-lg border border-white/10">
		<div class="w-full flex items-start align-middle justify-between">
			<div class="flex flex-col">
				<h1 class="text-3xl font-bold text-white drop-shadow">Docker Containers</h1>
				<p class="text-base text-gray-300 mb-6">Manage docker containers for <span class="font-semibold text-white">{data.system.label}</span> below.</p>
			</div>
		</div>

		<input
			type="text"
			placeholder="Search containers..."
			bind:value={search}
			class="mb-6 w-full px-4 py-2 rounded bg-black/30 border border-white/10 text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-green-400 transition"
		/>

		<div class="w-full space-y-4">
			{#each filteredContainers as container}
				<div class="w-full px-6 py-4 flex items-center justify-between glass-bg rounded-lg border border-white/10 shadow-sm transition hover:shadow-lg">
					<div class="flex flex-col items-start">
						<h2 class="text-lg font-bold text-white flex items-center gap-2">
							{container.name}
							<span class="text-xs font-mono {getStateStyle(container.state ?? '')} font-semibold bg-white/10 px-2 py-0.5 rounded">
								{container.state}
							</span>
						</h2>
						<p class="text-sm text-gray-400">ID: {container.dockerId.substring(0, 12)}</p>

						<div class="flex items-center gap-2 text-white/40 text-xs font-mono mt-1">
							<p>Created: {formatDate(container.createdAt ?? '')}</p>
							{#if container.containerMetrics}
								<span>•</span>
								<p>CPU: {(container.containerMetrics[0].cpuUsage || 0).toFixed(2)}%</p>
								<span>•</span>
								<p>Memory: {(container.containerMetrics[0].memoryUsage || 0).toFixed(2)} MB</p>
							{/if}
						</div>
					</div>

					<div class="flex items-center gap-2">
						{#if !container.state?.toLowerCase().includes("up")}
							<button
								class="flex align-middle items-center cursor-pointer gap-2 px-3 py-1 rounded-md border border-green-300 bg-green-300/10 active:scale-95 hover:bg-green-300/20 disabled:opacity-40 disabled:cursor-not-allowed transition"
								onclick={() => handleContainerAction(container.dockerId, 'start')}
							>
								Start
								<span class="icon-[mdi--play] w-5 h-5"></span>
							</button>
						{:else}
							<button
								class="flex align-middle items-center cursor-pointer gap-2 px-3 py-1 rounded-md border border-red-400 bg-red-400/10 active:scale-95 hover:bg-red-400/20 disabled:opacity-40 disabled:cursor-not-allowed transition"
								onclick={() => handleContainerAction(container.dockerId, 'stop')}
							>
								Stop
								<span class="icon-[mdi--stop] w-5 h-5"></span>
							</button>
							<button
								class="flex align-middle items-center cursor-pointer gap-2 px-3 py-1 rounded-md border border-neutral-600 bg-gray-300/10 active:scale-95 hover:bg-gray-300/20 disabled:opacity-40 disabled:cursor-not-allowed transition"
								onclick={() => handleContainerAction(container.dockerId, 'restart')}
							>
								Restart
								<span class="icon-[mdi--restart] w-5 h-5"></span>
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

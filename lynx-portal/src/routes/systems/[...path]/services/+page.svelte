<script lang="ts">
	import { toast } from 'svelte-sonner';

	const {data} = $props();

	let search = $state('');

	let filteredServices = $derived.by(() => {
		if (!search) return data.system.services;
		return data.system.services.filter(service =>
			service.name.toLowerCase().includes(search.toLowerCase()) ||
			(service.description && service.description.toLowerCase().includes(search.toLowerCase()))
		);
	})

	function serviceStateTag(state: string | null) : {text: string, color: string} {
		let color = '';
		switch (state ?? 'Unknown') {
			case 'Active':
			case 'Activating':
				color = 'text-green-400';
				break;
			case 'Inactive':
			case 'Reloading':
			case 'Deactivating':
				color = 'text-yellow-400';
				break;
			case 'Failed':
				color = 'text-red-400';
				break;
			default:
				color = 'text-gray-400';
		}
		return {text: state ?? "Unknown", color };
	}
</script>

<style>
    .glass-bg {
        background: rgba(30, 30, 30, 0.4);
        border: 1px solid rgba(255,255,255,0.08);
        box-shadow: 0 4px 32px 0 rgba(0,0,0,0.25);
        backdrop-filter: blur(1px);
    }
</style>

<div class="min-h-screen w-full flex flex-col items-center justify-start  py-12">
	<div class="flex items-center justify-between w-full max-w-4xl mb-6 px-6">
		<button class="flex items-center px-3 py-1 glass-bg rounded hover:bg-white/10 text-gray-200 border border-white/10 cursor-pointer" onclick={() => window.history.back()}>
			<span class="icon-[line-md--arrow-left] w-5 h-5"></span>
			Back
		</button>
		<h1 class="text-3xl font-bold text-white drop-shadow">Services</h1>
	</div>
	<div class="w-full max-w-4xl glass-bg rounded-xl p-8 shadow-lg border border-white/10">
		<p class="text-base text-gray-300 mb-6">Manage services for <span class="font-semibold text-white">{data.system.label}</span> below.</p>
		<input
			type="text"
			placeholder="Search services..."
			bind:value={search}
			class="mb-6 w-full px-4 py-2 rounded bg-black/30 border border-white/10 text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-green-400 transition"
		/>
		<div class="w-full space-y-4">
			{#each filteredServices as service}
				<div class="w-full px-6 py-4 flex items-center justify-between glass-bg rounded-lg border border-white/10 shadow-sm transition hover:shadow-lg">
					<div class="flex flex-col items-start">
						<h2 class="text-lg font-bold text-white flex items-center gap-2">
							{service.name}
							<span class="text-xs font-mono {serviceStateTag(service.state).color} font-semibold bg-white/10 px-2 py-0.5 rounded">{serviceStateTag(service.state).text}</span>
						</h2>
						<p class="text-sm text-gray-400">{service.description}</p>

						<div class="flex items-center gap-2 text-white/40 text-xs font-mono mt-1">
							<p>PID: {service.pid}</p>
							{#if service.cpu && !service.cpu.includes("unknown")}
								<span>•</span>
								<p>CPU: {service.cpu}</p>
							{/if}
							{#if service.memory && !service.memory.includes("unknown")}
								<span>•</span>
								<p>Memory: {service.memory}</p>
							{/if}
						</div>

					</div>
					<div class="flex items-center gap-2">
						{#if service.state != 'Active' && service.state != 'Activating'}
							<button
								disabled={service.state == 'Active' || service.state == 'Activating'}
								class="flex align-middle items-center cursor-pointer gap-2 px-3 py-1 rounded-md border border-green-300 bg-green-300/10 active:scale-95 hover:bg-green-300/20 disabled:opacity-40 disabled:cursor-not-allowed transition"
								onclick={() => { toast.success(`Starting ${service.name}...`); }}
							>
								Start
								<span class="icon-[mdi--play] w-5 h-5"></span>
							</button>
						{/if}
						{#if service.state == 'Active'}
							<button
								disabled={service.state != 'Active'}
								class="flex align-middle items-center cursor-pointer gap-2 px-3 py-1 rounded-md border border-red-400 bg-red-400/10 active:scale-95 hover:bg-red-400/20 disabled:opacity-40 disabled:cursor-not-allowed transition"
								onclick={() => { toast.success(`Stopping ${service.name}...`); }}
							>
								Stop
								<span class="icon-[mdi--stop]  w-5 h-5"></span>
							</button>
						{/if}
						{#if service.state == 'Active'}
							<button
								disabled={service.state != 'Active'}
								class="flex align-middle items-center cursor-pointer gap-2 px-3 py-1 rounded-md border border-neutral-600 bg-gray-300/10 active:scale-95 hover:bg-gray-300/20 disabled:opacity-40 disabled:cursor-not-allowed transition"
								onclick={() => { toast.success(`Restarting ${service.name}...`); }}
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
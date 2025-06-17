<script lang="ts">
	import '../app.css';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { onMount } from 'svelte';
	import AddServer from '$lib/components/AddServer.svelte';
	let activeTab = $state('');

	onMount(() => {
		// Set the active tab based on the current URL path
		activeTab = window.location.pathname.split('/')[1] || 'dashboard';
	})

	let { children, data } = $props();
</script>

<svelte:head>
	<title>lynx</title>
	<meta name="description" content="A SvelteKit application with a custom layout." />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
</svelte:head>

<div class="w-full h-full flex flex-col align-middle items-center justify-center p-20 gap-10">
	<nav class="w-full bg-[var(--foreground)] flex align-middle items-center justify-between p-3  rounded-md border border-[var(--border)]">
		<div class="flex items-center align-middle gap-10">
			<h1 class="text-[var(--primary)] text-xl font-bold">Lynx</h1>
			<div class="flex items-center align-middle gap-2">
				<a href="/" onclick={() => activeTab = 'dashboard'} class="{activeTab == 'dashboard' ? 'active' : ''}" >Dashboard</a>
				<a href="/systems" onclick={() => activeTab = 'systems'} class="{activeTab == 'systems' ? 'active' : ''}" >Systems</a>
				<a href="/alerts" onclick={() => activeTab = 'alerts'} class="{activeTab == 'alerts' ? 'active' : ''}" >Alerts</a>
			</div>
		</div>
		<div class="flex items-center align-middle gap-4">
			<AddServer />
			<Button
				variant="ghost"
				class="p-1 cursor-pointer hover:scale-105 active:scale-95 hover:text-[var(--primary)] active:text-[var(--primary)]"
				onclick={() => {
					window.location.href = '/settings';
				}}
			>
				<span class="icon-[line-md--cog-loop] w-6 h-6"></span>
			</Button>

		</div>
	</nav>
	{@render children()}
</div>


<style>


	nav {
			a {
				background: var(--background);
					padding-inline: 1rem;
					padding-block: 0.2rem;
					border-radius: 6px;
					border: 1px solid var(--border);
				color: var(--text);
					transition: all 0.2s ease;
			}

			a:hover {
					background: var(--background-alt);
					scale: 102%;
			}

			a:active {
					scale: 95%;
			}

			a.active {
					border-color: var(--primary);
					background: var(--secondary);
			}
	}

</style>
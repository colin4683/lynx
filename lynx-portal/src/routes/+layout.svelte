<script lang="ts">
	import '../app.css';
	let activeTab = $state('dashboard');
	let { children } = $props();
</script>

<svelte:head>
	<title>lynx</title>
	<meta name="description" content="A SvelteKit application with a custom layout." />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
</svelte:head>

<div class="w-full h-full flex flex-col align-middle items-center justify-center p-20 gap-10">
	<nav class="w-full bg-[var(--foreground)] flex align-middle items-center p-3 gap-10 rounded-md border border-[var(--border)]">
		<h1 class="text-[var(--primary)] text-xl font-bold">Lynx</h1>
		<div class="flex items-center align-middle gap-2">
			<a href="/" onclick={() => activeTab = 'dashboard'} class="{activeTab == 'dashboard' ? 'active' : ''}" >Dashboard</a>
			<a href="/systems" onclick={() => activeTab = 'systems'} class="{activeTab == 'systems' ? 'active' : ''}" >Systems</a>
			<a href="/alerts" onclick={() => activeTab = 'alerts'} class="{activeTab == 'alerts' ? 'active' : ''}" >Alerts</a>
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
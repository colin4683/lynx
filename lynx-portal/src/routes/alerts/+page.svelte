<script lang="ts">
	import { Label } from "$lib/components/ui/label/index.js";
	import { Switch } from "$lib/components/ui/switch/index.js";
	import * as Card from "$lib/components/ui/card/index.js";
	import { Button } from '$lib/components/ui/button';
	import { toast } from 'svelte-sonner';

	const { data } = $props();

	const rules = $derived.by(() => {
		return data.alerts ?? [];
	});




</script>
<div class="w-full flex items-center align-middle justify-between">
	<div>
		<h1 class="text-2xl font-bold">Alerts</h1>
		<p class="text-sm text-muted-foreground">Manage your alert rules below.</p>
	</div>
	<div>
		<Button variant="default" class="bg-primary/50 border border-primary cursor-pointer active:scale-95" onclick={() => window.location.href = '/alerts/new'}>
			<span class="icon-[line-md--plus-circle] w-5.5 h-5.5"></span>
			Add Alert Rule
		</Button>
	</div>
</div>
<div class="grid w-full grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 max-w-5xl">
	{#each rules as rule}
		<Card.Root class="relative bg-[var(--foreground)] p-4 rounded-lg shadow-md gap-1">
			<Card.Header class="bg-background rounded-lg border border-border items-center align-middle flex px-2 justify-between">
				<Card.Title class="text-lg font-semibold border-border cursor-pointer hover:text-primary transition-colors" onclick={() => {
					window.location.href = `/alerts/edit/${rule.id}`;
				}}>{rule.name}</Card.Title>
				<Switch  class="" checked={rule.active ?? false} />
			</Card.Header>
			<Card.Content>
				<p class="text-md">{rule.description}</p>
				<p class="text-sm text-muted-foreground">Created at: {new Date(rule.created ?? Date.now()).toLocaleString()}</p>

			</Card.Content>

		</Card.Root>
	{/each}
</div>

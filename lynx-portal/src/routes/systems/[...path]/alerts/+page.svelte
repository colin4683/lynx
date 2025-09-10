<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as Select from '$lib/components/ui/select';
	import { Switch } from '$lib/components/ui/switch';
	import { goto, invalidate, invalidateAll } from '$app/navigation';
	import { toast } from 'svelte-sonner';

	const { data } = $props();

	// State management for alert rules
	let appliedRules = $state(new Set(data.appliedAlerts?.map(alert => alert.id) || []));
	let availableRules = $state(data.availableAlerts || []);
	let showAddDialog = $state(false);
	let selectedRuleToAdd = $state([] as string[]);


	function getSeverityColor(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'text-red-500 bg-red-500/10';
			case 'high': return 'text-orange-500 bg-orange-500/10';
			case 'medium': return 'text-yellow-500 bg-yellow-500/10';
			case 'low': return 'text-green-500 bg-green-500/10';
			default: return 'text-gray-500 bg-gray-500/10';
		}
	}

	function getSeverityIcon(severity: string) {
		switch (severity?.toLowerCase()) {
			case 'critical': return 'icon-[heroicons--exclamation-triangle-solid]';
			case 'high': return 'icon-[heroicons--exclamation-circle-solid]';
			case 'medium': return 'icon-[heroicons--information-circle-solid]';
			case 'low': return 'icon-[heroicons--check-circle-solid]';
			default: return 'icon-[heroicons--question-mark-circle-solid]';
		}
	}

	function deleteAlertRule(ruleId: number) {
		// make request
		let formData = {
			alertId:ruleId,
			system: data.system?.id
		}
		fetch(`/systems/${data.system?.id}/alerts`, {
			method: 'DELETE',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify(formData)
		}).then(response => {
			if (response.ok) {
				// Update local state
				toast.success('Alert rule removed successfully');
				invalidate('app:system:alerts')
				invalidateAll();
				appliedRules.delete(ruleId);
				appliedRules = new Set(appliedRules); // Trigger reactivity
			} else {
				toast.error('Failed to remove alert rule');
			}
		})
	}

	function addAlertRule() {
		if (selectedRuleToAdd && selectedRuleToAdd.length > 0) {
			const ruleIds = selectedRuleToAdd.map(id => parseInt(id));
			const formJson = {
				alerts: ruleIds,
				system: data.system?.id
			};

			fetch(`/systems/${data.system?.id}/alerts`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify(formJson)
			}).then(response => {
				if (response.ok) {
					// Update local state
					toast.success('Alert rules added successfully');
					showAddDialog = false;
					invalidate('app:system:alerts')
					invalidateAll();
					appliedRules = new Set([...appliedRules, ...ruleIds]);
				} else {
					toast.error('Failed to add alert rules');
				}
			})
		}
	}
	const appliedAlerts = $derived(availableRules.filter(rule => appliedRules.has(rule.id)));
	const unappliedAlerts = $derived(availableRules.filter(rule => !appliedRules.has(rule.id)));
</script>

<div class="flex flex-col gap-6 w-full px-6 py-4 max-w-6xl mx-auto">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-4">
			<button
				class="icon-[ep--back] w-6 h-6 cursor-pointer text-muted-foreground hover:text-white transition-colors"
				onclick={() => goto(`/systems/${data.path}`)}
			></button>
			<div>
				<h1 class="text-2xl font-bold">Alert Management</h1>
				<p class="text-muted-foreground">Manage alert rules for {data.system?.label || data.system?.hostname}</p>
			</div>
		</div>
		<div class="flex gap-2">
			<Button variant="outline" onclick={() => showAddDialog = true}>
				<span class="icon-[heroicons--plus] w-4 h-4 mr-2"></span>
				Add Existing Rule
			</Button>
			<Button onclick={() => window.location.href = '/alerts/new'}>
				<span class="icon-[heroicons--plus] w-4 h-4 mr-2"></span>
				Create New Rule
			</Button>
		</div>
	</div>

	<!-- Applied Alert Rules -->
	<div class="space-y-4">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold">Applied Rules ({appliedAlerts.length})</h2>
			<Button variant="ghost" size="sm" onclick={() => goto('/alerts')}>
				View All Rules
			</Button>
		</div>

		{#if appliedAlerts.length === 0}
			<div class="rounded-lg border border-dashed border-border bg-muted/20 p-8 text-center bg-gradient-to-br from-[var(--background)]/60 via-[var(--foreground)]/60 to-[var(--background)]/60 backdrop-blur-xs">
				<span class="icon-[heroicons--shield-exclamation] w-12 h-12 mx-auto mb-4 text-muted-foreground opacity-50"></span>
				<h3 class="text-lg font-medium mb-2">No alert rules applied</h3>
				<p class="text-muted-foreground mb-4">This system has no alert rules configured. Add rules to start monitoring.</p>
				<Button onclick={() => showAddDialog = true}>
					Add Alert Rule
				</Button>
			</div>
		{:else}
			<div class="space-y-3">
				{#each appliedAlerts as rule}
					<div class="rounded-lg border border-border bg-gradient-to-br from-[var(--background)]/60 via-[var(--foreground)]/60 to-[var(--background)]/60 backdrop-blur-xs p-4 hover:bg-muted/20 transition-colors">
						<div class="flex items-start justify-between">
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-3 mb-2">
									<span class="w-2 h-2 rounded-full bg-green-500"></span>
									<h3 class="font-medium">{rule.name}</h3>
									<span class="inline-flex items-center gap-1 px-2 py-1 text-xs rounded-full {getSeverityColor(rule.severity)}">
										<span class="{getSeverityIcon(rule.severity)} w-3 h-3"></span>
										{rule.severity}
									</span>
								</div>

								{#if rule.description}
									<p class="text-sm text-muted-foreground mb-2">{rule.description}</p>
								{/if}

								<div class="flex items-center gap-4 text-xs text-muted-foreground">
									<span>Expression: <code class="px-1 py-0.5 rounded bg-muted font-mono">{rule.expression}</code></span>
								</div>
							</div>

							<div class="flex items-center gap-2 ml-4">
								<Button
									variant="ghost"
									class="cursor-pointer"
									size="sm"
									onclick={() => goto(`/alerts/edit/${rule.id}`)}
								>
									<span class="icon-[heroicons--pencil] w-4 h-4"></span>
								</Button>
								<Button
									variant="ghost"
									size="sm"

									class="text-red-500 hover:text-red-600 hover:bg-red-500/10 cursor-pointer"
									onclick={() => deleteAlertRule(rule.id)}
								>
									<span class="icon-[heroicons--x-mark] w-4 h-4"></span>
								</Button>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Recent Alert History -->
	<div class="space-y-4">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold">Recent Alert History</h2>
			<Button variant="ghost" size="sm" onclick={() => goto(`/systems/${data.path}/alerts/history`)}>
				View All History
			</Button>
		</div>

		{#if data.recentHistory?.length === 0}
			<div class="rounded-lg border border-border bg-card p-6 text-center">
				<span class="icon-[heroicons--clock] w-8 h-8 mx-auto mb-2 text-muted-foreground opacity-50"></span>
				<p class="text-muted-foreground">No recent alerts triggered</p>
			</div>
		{:else}
			<div class="rounded-lg border border-border bg-card divide-y divide-border bg-gradient-to-br from-[var(--background)]/60 via-[var(--foreground)]/60 to-[var(--background)]/60 backdrop-blur-xs">
				{#each data.recentHistory?.slice(0, 5) || [] as historyItem}
					<div class="p-4 flex items-center justify-between">
						<div class="flex items-center gap-3">
							<span class="{getSeverityIcon(historyItem.alertRule.severity)} w-4 h-4 {getSeverityColor(historyItem.alertRule.severity).split(' ')[0]}"></span>
							<div>
								<p class="font-medium">{historyItem.alertRule.name}</p>
								<p class="text-sm text-muted-foreground">{new Date(historyItem.date).toLocaleString()}</p>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

<!-- Add Existing Rule Dialog -->
<Dialog.Root bind:open={showAddDialog}>
	<Dialog.Content class="sm:max-w-md border-border">
		<Dialog.Header>
			<Dialog.Title>Add Existing Alert Rule</Dialog.Title>
			<Dialog.Description>
				Select an existing alert rule to apply to this system.
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4">
			{#if unappliedAlerts.length === 0}
				<p class="text-sm text-white m-3">No available alert rules. Create new alert rules first.</p>
			{:else}
				<Select.Root type="multiple" bind:value={selectedRuleToAdd}>
					<Select.Trigger>
						<span>Select an alert rule...</span>
					</Select.Trigger>
					<Select.Content class="bg-background border-border">
						{#each unappliedAlerts as rule}
							<Select.Item value={rule.id.toString()} class="bg-background outline-border border-border z-[10]" >
								<div class="flex items-center gap-2">
									<span class="{getSeverityIcon(rule.severity)} w-3 h-3 {getSeverityColor(rule.severity).split(' ')[0]}"></span>
									<span>{rule.name}</span>
									<span class="text-xs text-muted-foreground">({rule.severity})</span>
								</div>
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			{/if}

		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => showAddDialog = false}>Cancel</Button>
			<Button onclick={addAlertRule} disabled={selectedRuleToAdd.length == 0}>Add Rule(s)</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

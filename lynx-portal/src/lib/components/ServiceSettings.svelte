<script lang="ts">
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { Label } from "$lib/components/ui/label/index.js";
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { onMount } from 'svelte';

	type Settings = {
		trackAll: boolean;
		whitelist: string[];
		blacklist: string[];
	}

	let trackAll = $state(false);
	let whitelist: string[] = $state([]);
	let blacklist: string[] = $state([]);

	let whitelistInput = $state('');
	let blacklistInput = $state('');

	const STORAGE_KEY = 'serviceTrackerSettings';

	onMount(() => {
		try {
			const raw = localStorage.getItem(STORAGE_KEY);
			if (raw) {
				const parsed = JSON.parse(raw) as Settings;
				trackAll = !!parsed.trackAll;
				whitelist = Array.isArray(parsed.whitelist) ? parsed.whitelist : [];
				blacklist = Array.isArray(parsed.blacklist) ? parsed.blacklist : [];
			}
		} catch (e) {
			console.warn('Failed to load service tracker settings', e);
		}
	});

	function addToWhitelist() {
		const input = whitelistInput.trim();
		if (!input) return;

		// Allow comma or newline separated values
		const services = input.split(/[\n,]+/).map(s => s.trim()).filter(Boolean);
		for (const service of services) {
			if (!whitelist.includes(service) && service.length > 0) {
				whitelist = [...whitelist, service];
			}
		}
		whitelistInput = '';
	}

	function addToBlacklist() {
		const input = blacklistInput.trim();
		if (!input) return;

		// Allow comma or newline separated values
		const services = input.split(/[\n,]+/).map(s => s.trim()).filter(Boolean);
		for (const service of services) {
			if (!blacklist.includes(service) && service.length > 0) {
				blacklist = [...blacklist, service];
			}
		}
		blacklistInput = '';
	}

	function removeFromWhitelist(service: string) {
		whitelist = whitelist.filter(s => s !== service);
	}

	function removeFromBlacklist(service: string) {
		blacklist = blacklist.filter(s => s !== service);
	}

	function saveSettings() {
		const settings: Settings = { trackAll, whitelist, blacklist };
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
			console.log('Service tracker settings saved:', settings);
		} catch (e) {
			console.error('Failed to save service tracker settings:', e);
		}
	}
</script>

<Dialog.Root>
	<Dialog.Trigger class={"p-1 cursor-pointer hover:scale-105 active:scale-95 hover:text-[var(--primary)] active:text-[var(--primary)]"}
	>
		<span class="icon-[line-md--cog] w-8 h-8"></span>
	</Dialog.Trigger
	>
	<Dialog.Content class="sm:max-w-3xl border-border max-h-[80vh] overflow-y-auto">
		<Dialog.Header>
			<Dialog.Title>Manage systemctl tracker settings</Dialog.Title>
		</Dialog.Header>
		<div class="grid gap-6 py-4">
			<!-- Track All Services -->
			<div class="grid grid-cols-4 items-center gap-4">
				<Label for="track-all" class="text-right">Track all services</Label>
				<div class="flex items-center align-middle  gap-2 col-span-3">
					<Checkbox id="track-all" bind:checked={trackAll} />
					<p class="text-sm text-muted-foreground">When enabled, all system services will be tracked by default</p>
				</div>
			</div>

			<!-- Whitelist Services -->
			<div class="grid grid-cols-4 items-start gap-4">
				<Label for="whitelist" class="text-right pt-2">Whitelist services</Label>
				<div class="col-span-3 space-y-3">
					<p class="text-sm text-muted-foreground">
						Services in this list will always be tracked, even if "Track all services" is disabled.
						Enter service names separated by commas or new lines.
					</p>
					<div class="flex gap-2">
						<Input
							id="whitelist"
							placeholder="e.g. nginx.service, apache2.service"
							bind:value={whitelistInput}
							onkeydown={(e) => {
								if (e.key === 'Enter' && !e.shiftKey) {
									e.preventDefault();
									addToWhitelist();
								}
							}}
						/>
						<Button variant="outline" onclick={addToWhitelist} disabled={!whitelistInput.trim()}>
							Add
						</Button>
					</div>
					<div class="space-y-2">
						{#if whitelist.length > 0}
							<div class="flex flex-wrap gap-2">
								{#each whitelist as service}
									<span class="inline-flex items-center gap-2 px-2 py-1 rounded-md bg-green-100 dark:bg-green-900/30 border border-green-300 dark:border-green-700 text-sm">
										<span class="icon-[mdi--check-circle] w-4 h-4 text-green-600 dark:text-green-400"></span>
										{service}
										<button
											class="ml-1 text-red-500 hover:text-red-700 font-bold text-sm"
											onclick={() => removeFromWhitelist(service)}
											aria-label="Remove {service}"
										>
											×
										</button>
									</span>
								{/each}
							</div>
						{:else}
							<p class="text-sm text-muted-foreground italic">No whitelisted services</p>
						{/if}
					</div>
				</div>
			</div>

			<!-- Blacklist Services -->
			<div class="grid grid-cols-4 items-start gap-4">
				<Label for="blacklist" class="text-right pt-2">Blacklist services</Label>
				<div class="col-span-3 space-y-3">
					<p class="text-sm text-muted-foreground">
						Services in this list will never be tracked, even if "Track all services" is enabled.
						Enter service names separated by commas or new lines.
					</p>
					<div class="flex gap-2">
						<Input
							id="blacklist"
							placeholder="e.g. systemd-resolved.service, dbus.service"
							bind:value={blacklistInput}
							onkeydown={(e) => {
								if (e.key === 'Enter' && !e.shiftKey) {
									e.preventDefault();
									addToBlacklist();
								}
							}}
						/>
						<Button variant="outline" onclick={addToBlacklist} disabled={!blacklistInput.trim()}>
							Add
						</Button>
					</div>
					<div class="space-y-2">
						{#if blacklist.length > 0}
							<div class="flex flex-wrap gap-2">
								{#each blacklist as service}
									<span class="inline-flex items-center gap-2 px-2 py-1 rounded-md bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 text-sm">
										<span class="icon-[mdi--close-circle] w-4 h-4 text-red-600 dark:text-red-400"></span>
										{service}
										<button
											class="ml-1 text-red-500 hover:text-red-700 font-bold text-sm"
											onclick={() => removeFromBlacklist(service)}
											aria-label="Remove {service}"
										>
											×
										</button>
									</span>
								{/each}
							</div>
						{:else}
							<p class="text-sm text-muted-foreground italic">No blacklisted services</p>
						{/if}
					</div>
				</div>
			</div>
		</div>
		<Dialog.Footer>
			<Button type="button" class="text-black cursor-pointer" onclick={saveSettings}>Save changes</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
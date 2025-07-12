<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input/index.js';

	let command = $state('');
	let output = $state<string[]>([]);
	let executing = $state(false);
	let scrollContainer: HTMLDivElement | null = null;

	function executeCommand() {
		if (!command.trim()) return;

		output = [];
		executing = true;

		const commands: Record<string, string[]> = {
			'ls': [
				'Desktop  Documents  Downloads  Music  Pictures  Public  Templates  Videos',
				'',
				'Total items: 8'
			],
			'ps aux': [
				'USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND',
				'root         1  0.0  0.0 169316 13168 ?        Ss   Jul10   0:03 /sbin/init',
				'root         2  0.0  0.0      0     0 ?        S    Jul10   0:00 [kthreadd]',
				'user      1234  1.2  2.1 245678 45678 ?        Sl   Jul10   5:23 /usr/lib/firefox/firefox',
				'',
				'Total processes: 127'
			],
			'ping example.com': [
				'PING example.com (93.184.216.34) 56(84) bytes of data.',
				'64 bytes from 93.184.216.34: icmp_seq=1 ttl=54 time=12.3 ms',
				'64 bytes from 93.184.216.34: icmp_seq=2 ttl=54 time=11.8 ms',
				'64 bytes from 93.184.216.34: icmp_seq=3 ttl=54 time=13.2 ms',
				'64 bytes from 93.184.216.34: icmp_seq=4 ttl=54 time=12.5 ms',
				'64 bytes from 93.184.216.34: icmp_seq=1 ttl=54 time=12.3 ms',
				'64 bytes from 93.184.216.34: icmp_seq=2 ttl=54 time=11.8 ms',
				'64 bytes from 93.184.216.34: icmp_seq=3 ttl=54 time=13.2 ms',
				'64 bytes from 93.184.216.34: icmp_seq=4 ttl=54 time=12.5 ms',
				'64 bytes from 93.184.216.34: icmp_seq=1 ttl=54 time=12.3 ms',
				'64 bytes from 93.184.216.34: icmp_seq=2 ttl=54 time=11.8 ms',
				'64 bytes from 93.184.216.34: icmp_seq=3 ttl=54 time=13.2 ms',
				'64 bytes from 93.184.216.34: icmp_seq=4 ttl=54 time=12.5 ms'
			]
		}

		const selectedOutput = commands[command] || [
			`Command not found: ${command}`,
			'',
			'Available test commands: ls, ps aux, ping example.com'
		];

		let i = 0;
		const interval = setInterval(() => {
			if (i < selectedOutput.length) {
				output = [...output, selectedOutput[i]];
				i++;
			} else {
				clearInterval(interval);
				executing = false;
			}
		}, 200);
	}

	$effect(() => {
		if (output.length > 0 && scrollContainer) {
			scrollContainer.scrollTop = scrollContainer.scrollHeight;
		}
	});

</script>

<Dialog.Root>
	<Dialog.Trigger class={"flex items-center justify-center"}
	>
		<span class="icon-[lucide--square-terminal] text-white/80 w-6 h-6 cursor-pointer hover:text-primary transition-colors active:scale-95"></span>
	</Dialog.Trigger
	>

	<Dialog.Content class="sm:max-w-2xl border-border">
		<Dialog.Header>
			<Dialog.Title>Command Stream</Dialog.Title>
			<Dialog.Description>
				Stream output of command from agent
			</Dialog.Description>
		</Dialog.Header>

		<div class="w-full flex flex-col items-start gap-2">
			<div class="flex items-center align-middle gap-1.5">
				<Input bind:value={command} type="text" placeholder="command to stream" class="outline-none active:outline-none focus:outline-none focus:ring-0 ring-0  active:ring-0" />
				<Button onclick={() => executeCommand()} disabled={executing} variant="outline">{executing ? 'Running...' : 'Execute'}</Button>
			</div>

			<div
				bind:this={scrollContainer}
				class="w-full min-h-[300px] max-h-[300px] overflow-auto scroll-auto font-mono text-sm overscroll-auto bg-foreground border border-border p-2">
				{#if output.length === 0}
					<div class="text-muted-foreground">Output will appear here...</div>
				{:else}
					{#each output as line}
						<div>{line}</div>
					{/each}
					{#if executing}
						<div class="animate-pulse">█</div>
					{/if}
				{/if}
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
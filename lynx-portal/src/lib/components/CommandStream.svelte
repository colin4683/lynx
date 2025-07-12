<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input/index.js';
	import { onDestroy } from 'svelte';

	let command = $state('');
	let output = $state<string[]>([]);
	let executing = $state(false);
	let scrollContainer: HTMLDivElement | null = null;
	let socket: WebSocket | null = null;
	function executeCommand() {
		if (!command.trim()) return;

		output = [`$ ${command}`];
		executing = true;

		if (socket) socket.close();

		socket = new WebSocket("ws://127.0.0.1:8080");

		socket.onopen = () => {
			socket?.send(JSON.stringify({
				type: 'execute',
				command: command.split(' ')[0],
				args: command.split(' ').slice(1)
			}));
		}

		socket.onerror = (error) => {
			console.error('WebSocket error:', error);
			executing = false;
			socket?.close();
		}

		socket.onclose = () => {
			if (executing) {
				executing = false;
				output = [...output, 'Connection closed unexpectedly.'];
			}
		}

		socket.onmessage = (event) => {
			// data is just raw text
			const data = event.data;
			if (data === 'EOF') {
				executing = false;
				socket?.send(JSON.stringify({ type: 'stop' }));
				setTimeout(() => {
					socket?.close()
				}, 500);
			} else {
				output = [...output, data];
			}
		}
	}

	function stopCommand() {
		if (socket && socket.readyState === WebSocket.OPEN) {
			socket.send(JSON.stringify({ type: 'stop' }));
			executing = false;
			setTimeout(() => {
				socket?.close();
			}, 500);
		}
	}

	function destroy() {
		stopCommand();
		if (socket) {
			socket.close();
			socket = null;
		}
		executing = false;
		output = [];
		command = '';
	}

	$effect(() => {
		if (output.length > 0 && scrollContainer) {
			scrollContainer.scrollTop = scrollContainer.scrollHeight;
		}
	});



	onDestroy(() => {
		destroy();
	})

</script>

<Dialog.Root onOpenChange={(open) => {
		if (!open) {
			destroy();
		}
	}}>
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
				<Button onclick={() => stopCommand()} disabled={!executing} variant="destructive">Stop</Button>
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
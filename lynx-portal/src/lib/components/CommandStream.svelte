<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { onDestroy } from 'svelte';
	import { DialogHeader, DialogTitle } from '$lib/components/ui/dialog/index.js';
	import { fly, fade } from 'svelte/transition';
	import { tick } from 'svelte';

	let command = $state('');
	let output = $state<string[]>([]);
	let executing = $state(false);
	let scrollContainer: HTMLDivElement | null = null;
	let socket: WebSocket | null = null;
	let inputValue = $state('');
	let inputRef: HTMLInputElement | null = null;

	function executeCommand() {
		if (!command.trim()) return;

		output = [...output, `$ ${command}`];
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
			let displayData: string;
			try {
				const parsed = JSON.parse(data);
				displayData = typeof parsed === 'object' ? JSON.stringify(parsed, null, 2) : String(parsed);
			} catch {
				displayData = String(data);
			}
			if (data === 'EOF') {
				executing = false;
				socket?.send(JSON.stringify({ type: 'stop' }));
				setTimeout(() => {
					socket?.close()
				}, 500);
			} else {
				console.log('Received data:', data);
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

	async function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !executing) {
			command = inputValue;
			executeCommand();
			inputValue = '';
			await tick();
			scrollToBottom();
		} else if (e.key === 'c' && e.ctrlKey && executing) {
			stopCommand();
			output = [...output, '^C'];
			executing = false;
			await tick();
			scrollToBottom();
		} else if (e.key === 'l' && e.ctrlKey) {
			e.preventDefault();
			output = [];
			command = '';
		} else if (e.key === 'Escape' || (e.key === 'd' && e.ctrlKey)) {
			e.preventDefault();
			destroy();
		}
	}

	function scrollToBottom() {
		if (scrollContainer) {
			scrollContainer.scrollTop = scrollContainer.scrollHeight;
		}
	}

	$effect(() => {
		if (inputRef) inputRef.focus();
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

	<Dialog.Content class="sm:max-w-6xl h-[60%] bg-background flex flex-col border-border">
		<DialogHeader>
			<Dialog.Title>Command Stream</Dialog.Title>
			<Dialog.Description>
				Stream output of command from agent
			</Dialog.Description>
		</DialogHeader>

		<div class="w-full h-full flex flex-col items-start justify-start gap-2 pb-2">
			<div
				bind:this={scrollContainer}
				class="w-full h-full overflow-scroll font-mono text-sm overscroll-auto bg-foreground border border-border p-2 rounded transition-all duration-300">
				{#each output as line}
					<div in:fly={{ y: 10, duration: 50 }} out:fade={{duration: 100}}>{line}</div>
				{/each}
				{#if executing}
					<div class="animate-pulse">█</div>
				{/if}
				<!-- Terminal prompt -->
				<div class="flex items-center w-full mt-2">
					<span class="text-primary font-bold select-none">$</span>
					<input
						bind:this={inputRef}
						bind:value={inputValue}
						class="bg-transparent border-none outline-none text-white font-mono w-full ml-2 animate-in fade-in {executing ? 'opacity-60 cursor-not-allowed' : ''}"
						placeholder="Type a command and press Enter..."
						readonly={executing}
						onkeydown={handleKeydown}
					/>
				</div>
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
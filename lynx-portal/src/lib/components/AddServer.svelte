<script lang="ts">
	import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
	import * as Dialog from "$lib/components/ui/dialog/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { Label } from "$lib/components/ui/label/index.js";
</script>

<Dialog.Root>
	<Dialog.Trigger class={buttonVariants({ variant: "outline" })}
	>
		<span class="icon-[line-md--plus-circle] w-5.5 h-5.5"></span>
		Server
	</Dialog.Trigger
	>
	<Dialog.Content class="sm:max-w-lg border-border">
		<Dialog.Header>
			<Dialog.Title>Deploy an agent</Dialog.Title>
			<Dialog.Description>
				Please fill in the details below to generate an installation script for the agent.
			</Dialog.Description>
		</Dialog.Header>
		<div class="grid gap-4 py-4">
			<div class="grid grid-cols-4 items-center gap-4">
				<Label for="label" class="text-right">System label</Label>
				<Input id="label" placeholder="ubuntu-server-1" class="col-span-3" />
			</div>
			<div class="grid grid-cols-4 items-center gap-4">
				<Label for="host" class="text-right">Host</Label>
				<Input id="host" placeholder="127.0.0.1" class="col-span-3" />
			</div>
			<div class="grid grid-cols-4 items-center gap-4">
				<Label for="host" class="text-right">Host</Label>
				<Input id="host" placeholder="127.0.0.1" class="col-span-3" />
			</div>
		</div>
		<Dialog.Footer>
			<Button type="submit" class="text-black">Save changes</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
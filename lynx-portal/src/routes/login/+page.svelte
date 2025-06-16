<script lang="ts">
	import { Button } from "$lib/components/ui/button/index.js";
	import * as Card from "$lib/components/ui/card/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { Label } from "$lib/components/ui/label/index.js";

	import {enhance} from '$app/forms';

	import type {ActionData} from "./$types";

	export let form: ActionData;

</script>


<Card.Root class="mx-auto w-full max-w-lg border-[var(--border)] bg-[var(--foreground)]">
	<Card.Header>
		<Card.Title class="text-2xl">Login</Card.Title>
		<Card.Description>Please authorize your credentials before accessing the dashboard.</Card.Description>
	</Card.Header>

	<Card.Content>
		<form method="POST" use:enhance>
			<div class="grid gap-4">
				<div class="grid gap-2">
					<Label for="email">Email</Label>
					<Input id="email" name="email" type="email" placeholder="m@example.com" value={form?.email} required />
				</div>
				<div class="grid gap-2">
					<div class="flex items-center">
						<Label for="password">Password</Label>
						<a href="##" class="ml-auto inline-block text-xs underline">
							Forgot your password?
						</a>
					</div>
					<Input id="password" name="password" type="password" required />
				</div>
				<p class="text-red-400 transition-all">{form?.message ?? ""}</p>
				<Button type="submit" class="w-full bg-[var(--primary)]/50 border border-[var(--primary)] hover:bg-[var(--secondary)] hover:text-[var(--primary)] cursor-pointer active:scale-95 transition-all">Login</Button>
			</div>
		</form>
	</Card.Content>
</Card.Root>
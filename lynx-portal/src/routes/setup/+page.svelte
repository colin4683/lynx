<script lang="ts">
import * as Card from '$lib/components/ui/card';
import { Button } from '$lib/components/ui/button';
import { Label } from '$lib/components/ui/label';
import { Input } from '$lib/components/ui/input';
import {enhance} from '$app/forms';

import type {ActionData} from "./$types";

export let form: ActionData;
</script>

<Card.Root class="mx-auto w-full max-w-lg border-[var(--border)] bg-[var(--foreground)]">
	<Card.Header>
		<Card.Title class="text-2xl">Setup your admin account</Card.Title>
		<Card.Description>To start using lynx, provide an email and password for your admin account.</Card.Description>
	</Card.Header>

	<Card.Content>
		<form method="POST" use:enhance>
			<div class="grid gap-4">
				<div class="grid gap-2">
					<Label for="email">Email</Label>
					<Input id="email" name="email" type="email" placeholder="email@example.com" value={form?.email} required />
				</div>
				<div class="grid gap-2">
					<div class="flex items-center">
						<Label for="password">Password</Label>
					</div>
					<Input id="password" name="password" type="password" required />
					<div class="flex items-center">
						<Label for="password">Confirm Password</Label>
					</div>
					<Input id="confirm-password" name="confirm-password" type="password" required />
				</div>
				<p class="text-red-400 transition-all">{form?.message ?? ""}</p>
				<Button type="submit" class="w-full bg-[var(--primary)]/50 border border-[var(--primary)] hover:bg-[var(--secondary)] hover:text-[var(--primary)] cursor-pointer active:scale-95 transition-all">Setup</Button>
			</div>
		</form>
	</Card.Content>
</Card.Root>
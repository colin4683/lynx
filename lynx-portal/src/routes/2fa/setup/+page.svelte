<script lang="ts">
	import { enhance } from "$app/forms";
	import * as Card from '$lib/components/ui/card';
	import type { ActionData, PageData } from "./$types";
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';

	export let data: PageData;
	export let form: ActionData;
</script>

<Card.Root class="mx-auto w-full max-w-lg border-[var(--border)] bg-[var(--foreground)]">
	<Card.Header>
		<Card.Title class="text-2xl">2fa Setup</Card.Title>
		<Card.Description>Please scan the QR code and enter the code provided.</Card.Description>
	</Card.Header>

	<Card.Content>
		<div class="w-full flex justify-center align-middle items-center">
			<div style="width:200px; height: 200px;">
				{@html data.qrcode}
			</div>
		</div>
		<form method="POST" use:enhance>
			<div class="grid gap-4">
				<input name="key" value={data.encodedTOTPKey} hidden required />
				<div class="grid gap-2">
					<Label for="form-totp.code">Code</Label>
					<Input id="form-totp.code" name="code" type="text" placeholder="" required />
				</div>
				<p class="text-red-400 transition-all">{form?.message ?? ""}</p>
				<Button type="submit" class="w-full bg-[var(--primary)]/50 border border-[var(--primary)] hover:bg-[var(--secondary)] hover:text-[var(--primary)] cursor-pointer active:scale-95 transition-all">Submit</Button>
			</div>
		</form>
	</Card.Content>
</Card.Root>
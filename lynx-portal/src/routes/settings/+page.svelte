<script lang="ts">
	import { CardTitle, CardDescription, CardHeader, CardContent, CardFooter, Card } from "$lib/components/ui/card";
	import { Input } from "$lib/components/ui/input";
	import { Button } from "$lib/components/ui/button";
	import { Checkbox } from "$lib/components/ui/checkbox";


	let currentTab = $state('account');

</script>

<div class="w-full max-w-6xl">
	<div class="grid w-full max-w-6xl gap-2 bg-[var(--foreground)] py-3 pl-2 border rounded-md rounded-b-none border-border">
		<h1 class="text-3xl flex items-center gap-3 font-semibold">
			<span class="icon-[line-md--cog-loop] w-8 h-8"></span>
			Settings
		</h1>
	</div>
	<div class=" grid w-full max-w-6xl items-start gap-6 md:grid-cols-[180px_1fr] lg:grid-cols-[250px_1fr] bg-[var(--foreground)]/30 p-3 border-b border-x rounded-md rounded-t-none border-border transition-all">
		<nav class="text-muted-foreground grid gap-4 text-sm">
			<a class={`settings-nav-item ${currentTab == "account" ? "active" : ""}`} onclick={() => currentTab = "account"}>Account</a>
			<a class={`settings-nav-item ${currentTab == "agents" ? "active" : ""}`} onclick={() => currentTab = "agents"}>Agents</a>
			<a class={`settings-nav-item ${currentTab == "notifs" ? "active" : ""}`}  onclick={() => currentTab = "notifs"}>Notifications</a>
			<a class={`settings-nav-item ${currentTab == "security" ? "active" : ""}`}  onclick={() => currentTab = "security"}>Security</a>
		</nav>


		<div class="w-full flex flex-col gap-4">
			{#if currentTab === 'account'}
				<Card class="w-full">
					<CardHeader>
						<CardTitle>Account Settings</CardTitle>
						<CardDescription>Manage your account settings and preferences.</CardDescription>
					</CardHeader>
					<CardContent class="space-y-6">
						<div class="grid gap-4">
							<Input placeholder="Email" />
							<Input type="password" placeholder="Password" />
						</div>
					</CardContent>
					<CardFooter>
						<Button>Save Changes</Button>
					</CardFooter>
				</Card>
			{/if}
			<!-- Add other tabs content here -->
		</div>
	</div>
</div>


<style>
	.settings-nav-item {
			cursor: pointer;
			color: var(--muted-foreground);
			transition: all 0.2s ease;
			transform: translateX(0px);
	}

	.settings-nav-item.active {
			color: var(--primary);
			font-weight: 600;
	}

	.settings-nav-item:hover {
			color: rgba(168, 237, 131, 0.48);
			transform:translateX(6px);
	}
</style>
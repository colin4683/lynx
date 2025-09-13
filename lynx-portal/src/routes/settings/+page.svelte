<script lang="ts">
	import { CardTitle, CardDescription, CardHeader, CardContent, CardFooter, Card } from "$lib/components/ui/card";
	import { Input } from "$lib/components/ui/input";
	import { Button } from "$lib/components/ui/button";
	import { Label } from "$lib/components/ui/label";
	import { Switch } from "$lib/components/ui/switch";
	import { toast } from 'svelte-sonner';

	let { data } = $props();

	let currentTab = $state('account');
	let loaded = $state(false);
	// Account settings
	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let enableTwoFactor = $state(true);

	// Notification settings - Shoutrrr URL schema format
	let enabledNotifiers = $state<string[]>([]);

	// Gmail/SMTP settings - smtp://username:password@host:port/?from=fromAddress&to=recipient1[,recipient2,...]
	let emailEnabled = $state(false);
	let smtpUsername = $state('');
	let smtpPassword = $state('');
	let smtpHost = $state('');
	let smtpPort = $state('587');
	let emailFrom = $state('');
	let emailTo = $state('');

	// Slack settings - slack://[botname@]token-a/token-b/token-c
	let slackEnabled = $state(false);
	let slackBotName = $state('');
	let slackTokenA = $state('');
	let slackTokenB = $state('');
	let slackTokenC = $state('');

	// Discord settings - discord://token@id?username=botname
	let discordEnabled = $state(false);
	let discordToken = $state('');
	let discordChannelId = $state('');
	let discordBotName = $state('');

	// Telegram settings - telegram://token@telegram?chats=@channel-1[,chat-id-1,...]
	let telegramEnabled = $state(false);
	let telegramToken = $state('');
	let telegramChats = $state('');

	// Derived Shoutrrr URLs for each notification type
	const emailUrl = $derived.by(() => {
		if (!emailEnabled || !smtpUsername || !smtpPassword || !smtpHost || !emailFrom || !emailTo) {
			return '';
		}
		const url = new URL(`smtp://${encodeURIComponent(smtpUsername)}:${encodeURIComponent(smtpPassword)}@${smtpHost}:${smtpPort}/`);
		url.searchParams.set('from', emailFrom);
		url.searchParams.set('to', emailTo);
		return url.toString();
	});

	const slackUrl = $derived.by(() => {
		if (!slackEnabled || !slackTokenA || !slackTokenB || !slackTokenC) {
			return '';
		}
		const botPrefix = slackBotName ? `${encodeURIComponent(slackBotName)}@` : '';
		return `slack://${botPrefix}${slackTokenA}/${slackTokenB}/${slackTokenC}`;
	});

	const discordUrl = $derived.by(() => {
		if (!discordEnabled || !discordToken || !discordChannelId) {
			return '';
		}
		const url = new URL(`discord://${discordToken}@${discordChannelId}`);
		if (discordBotName) {
			url.searchParams.set('username', discordBotName);
		}
		return url.toString();
	});

	const telegramUrl = $derived.by(() => {
		if (!telegramEnabled || !telegramToken || !telegramChats) {
			return '';
		}
		const url = new URL(`telegram://${telegramToken}@telegram`);
		url.searchParams.set('chats', telegramChats);
		return url.toString();
	});

	// Initialize from existing data
	$effect(() => {
		if (data.notifiers && !loaded) {
			loaded = true;
			console.log('Loaded notifiers:', data.notifiers);
			data.notifiers.forEach(notifier => {
				try {
					const url = new URL(notifier.value);
					const isActive = enabledNotifiers.includes(notifier.type);

					switch (url.protocol) {
						case 'smtp:':
							emailEnabled = true;
							smtpUsername = decodeURIComponent(url.username || '');
							smtpPassword = decodeURIComponent(url.password || '');
							smtpHost = url.hostname;
							smtpPort = url.port || '587';
							emailFrom = url.searchParams.get('from') || '';
							emailTo = url.searchParams.get('to') || '';
							break;

						case 'slack:':
							slackEnabled = true;
							const slackPath = url.pathname.substring(1); // Remove leading /
							const slackParts = slackPath.split('/');

							// Check if first part contains bot name
							if (url.username) {
								slackBotName = decodeURIComponent(url.username);
								slackTokenA = slackParts[0] || '';
								slackTokenB = slackParts[1] || '';
								slackTokenC = slackParts[2] || '';
							} else {
								slackTokenA = slackParts[0] || '';
								slackTokenB = slackParts[1] || '';
								slackTokenC = slackParts[2] || '';
							}
							break;

						case 'discord:':
							discordEnabled = true;
							discordToken = url.username || '';
							discordChannelId = url.hostname || '';
							discordBotName = url.searchParams.get('username') || '';
							break;

						case 'telegram:':
							telegramEnabled = true;
							telegramToken = url.username || '';
							telegramChats = url.searchParams.get('chats') || '';
							break;
					}
				} catch (error) {
					console.warn(`Failed to parse notifier ${notifier.id}:`, error);
				}
			});
		}
	});

	// Functions to toggle individual notification types
	function toggleEmailNotifications(enabled: boolean) {
		emailEnabled = enabled;
		if (enabled) {
			if (!enabledNotifiers.includes('email')) {
				enabledNotifiers = [...enabledNotifiers, 'email'];
			}
		} else {
			enabledNotifiers = enabledNotifiers.filter(type => type !== 'email');
		}
	}

	function toggleSlackNotifications(enabled: boolean) {
		slackEnabled = enabled;
		if (enabled) {
			if (!enabledNotifiers.includes('slack')) {
				enabledNotifiers = [...enabledNotifiers, 'slack'];
			}
		} else {
			enabledNotifiers = enabledNotifiers.filter(type => type !== 'slack');
		}
	}

	function toggleDiscordNotifications(enabled: boolean) {
		discordEnabled = enabled;
		if (enabled) {
			if (!enabledNotifiers.includes('discord')) {
				enabledNotifiers = [...enabledNotifiers, 'discord'];
			}
		} else {
			enabledNotifiers = enabledNotifiers.filter(type => type !== 'discord');
		}
	}

	function toggleTelegramNotifications(enabled: boolean) {
		telegramEnabled = enabled;
		if (enabled) {
			if (!enabledNotifiers.includes('telegram')) {
				enabledNotifiers = [...enabledNotifiers, 'telegram'];
			}
		} else {
			enabledNotifiers = enabledNotifiers.filter(type => type !== 'telegram');
		}
	}

	// Alert settings
	let defaultSeverity = $state('medium');
	let alertRetention = $state('30');
	let enableAlertHistory = $state(true);
	let alertCooldown = $state('5');

	// System settings
	let systemTimeout = $state('30');
	let metricRetention = $state('90');
	let enableSystemLogging = $state(true);
	let maxSystemsPerUser = $state('50');

	// Dashboard settings
	let defaultRefreshRate = $state('30');
	let enableAutoRefresh = $state(true);
	let darkMode = $state(true);
	let compactView = $state(false);

	// Agent settings
	let agentUpdateInterval = $state('60');
	let enableAgentLogging = $state(true);
	let maxAgentConnections = $state('100');

	function saveAccountSettings() {
		console.log('Saving account settings...');
	}

	function saveNotificationSettings() {
		const notifications = [];

		if (emailEnabled && emailUrl) {
			notifications.push({ type: 'email', url: emailUrl });
		}
		if (slackEnabled && slackUrl) {
			notifications.push({ type: 'slack', url: slackUrl });
		}
		if (discordEnabled && discordUrl) {
			notifications.push({ type: 'discord', url: discordUrl });
		}
		if (telegramEnabled && telegramUrl) {
			notifications.push({ type: 'telegram', url: telegramUrl });
		}

		fetch("/settings", {
			method: "POST",
			headers: {
				"Content-Type": "application/json"
			},
			body: JSON.stringify({ notifiers: notifications })
		}).then(response => {
			if (response.ok) {
				toast.success("Notification settings saved successfully.");
			} else {
				toast.error("Failed to save notification settings.");
			}
		}).catch(error => {
			console.error("Error saving notification settings:", error);
			toast.error("An error occurred while saving notification settings.");
		})
	}

	function saveAlertSettings() {
		console.log('Saving alert settings...');
	}

	function saveSystemSettings() {
		console.log('Saving system settings...');
	}

	function saveDashboardSettings() {
		console.log('Saving dashboard settings...');
	}

	function saveAgentSettings() {
		console.log('Saving agent settings...');
	}
</script>

<div class="flex flex-col min-h-[600px] w-full max-w-6xl bg-background">
	<!-- Header -->
	<div class="flex-shrink-0 border-b border-border bg-card">
		<div class="container mx-auto px-6 py-4">
			<h1 class="text-3xl font-bold tracking-tight flex items-center gap-3">
				<span class="icon-[line-md--cog-loop] w-8 h-8"></span>
				Settings
			</h1>
			<p class="text-muted-foreground mt-1">Manage your account and application preferences</p>
		</div>
	</div>

	<!-- Main Content -->
	<div class="flex-1 min-h-0 container mx-auto px-6 py-6">
		<div class="flex gap-6 h-full">
			<!-- Navigation Sidebar -->
			<div class="flex-shrink-0 w-64">
				<nav class="space-y-1">
					<button
						class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer {currentTab === 'account' ? 'bg-primary/60 border border-primary text-primary-foreground' : 'text-muted-foreground hover:text-white/60 hover:bg-foreground'}"
						onclick={() => currentTab = 'account'}
					>
						<span class="icon-[heroicons--user] w-4 h-4 inline mr-2"></span>
						Account
					</button>
					<button
						class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer {currentTab === 'notifications' ? 'bg-primary/60 border border-primary text-primary-foreground' : 'text-muted-foreground hover:text-white/60 hover:bg-foreground'}"
						onclick={() => currentTab = 'notifications'}
					>
						<span class="icon-[heroicons--bell] w-4 h-4 inline mr-2"></span>
						Notifications
					</button>
					<button
						class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer {currentTab === 'alerts' ? 'bg-primary/60 border border-primary text-primary-foreground' : 'text-muted-foreground hover:text-white/60 hover:bg-foreground'}"
						onclick={() => currentTab = 'alerts'}
					>
						<span class="icon-[heroicons--shield-exclamation] w-4 h-4 inline mr-2"></span>
						Alert Rules
					</button>
					<button
						class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer {currentTab === 'systems' ? 'bg-primary/60 border border-primary text-primary-foreground' : 'text-muted-foreground hover:text-white/60 hover:bg-foreground'}"
						onclick={() => currentTab = 'systems'}
					>
						<span class="icon-[heroicons--server] w-4 h-4 inline mr-2"></span>
						Systems
					</button>
					<button
						class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer {currentTab === 'dashboard' ? 'bg-primary/60 border border-primary text-primary-foreground' : 'text-muted-foreground hover:text-white/60 hover:bg-foreground'}"
						onclick={() => currentTab = 'dashboard'}
					>
						<span class="icon-[heroicons--squares-2x2] w-4 h-4 inline mr-2"></span>
						Dashboard
					</button>
					<button
						class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium cursor-pointer transition-colors {currentTab === 'agents' ? 'bg-primary/60 border border-primary text-primary-foreground' : 'text-muted-foreground hover:text-white/60 hover:bg-foreground'}"
						onclick={() => currentTab = 'agents'}
					>
						<span class="icon-[heroicons--cpu-chip] w-4 h-4 inline mr-2"></span>
						Agents
					</button>
				</nav>
			</div>

			<!-- Settings Content -->
			<div class="flex-1 w-full h-[600px]">
				<div class="h-full border border-border rounded-lg bg-card">
					<div class="h-full flex flex-col">
						<!-- Content Area - Scrollable -->
						<div class="flex-1 overflow-y-auto p-4">
							{#if currentTab === 'account'}
								<div class="space-y-6">
									<div>
										<h2 class="text-xl font-semibold mb-2">Account Settings</h2>
										<p class="text-muted-foreground text-sm">Manage your account security and preferences</p>
									</div>

									<div class="grid grid-cols-1  gap-4">
										<Card>
											<CardHeader>
												<CardTitle>Account Information</CardTitle>
												<CardDescription>View your account details</CardDescription>
											</CardHeader>
											<CardContent class="space-y-3">
												<div class="flex justify-between">
													<span class="text-muted-foreground">Email:</span>
													<span>{data.user?.email || 'user@example.com'}</span>
												</div>
												<div class="flex justify-between">
													<span class="text-muted-foreground">Account Type:</span>
													<span>User</span>
												</div>
												<div class="flex justify-between">
													<span class="text-muted-foreground">Email Verified:</span>
													<span class="flex items-center gap-1">
														{data.user?.emailVerified ? 'Yes' : 'No'}
														{#if data.user?.emailVerified}
															<span class="icon-[heroicons--check-circle] w-4 h-4 text-green-500"></span>
														{:else}
															<span class="icon-[heroicons--x-circle] w-4 h-4 text-red-500"></span>
														{/if}
													</span>
												</div>
											</CardContent>
										</Card>
										<Card>
											<CardHeader>
												<CardTitle>Change Password</CardTitle>
												<CardDescription>Update your account password</CardDescription>
											</CardHeader>
											<CardContent class="space-y-3">
												<div>
													<Label for="current-password">Current Password</Label>
													<Input id="current-password" type="password" bind:value={currentPassword} />
												</div>
												<div>
													<Label for="new-password">New Password</Label>
													<Input id="new-password" type="password" bind:value={newPassword} />
												</div>
												<div>
													<Label for="confirm-password">Confirm New Password</Label>
													<Input id="confirm-password" type="password" bind:value={confirmPassword} />
												</div>
											</CardContent>
											<CardFooter>
												<Button onclick={saveAccountSettings}>Update Password</Button>
											</CardFooter>
										</Card>
									</div>
								</div>


							{:else if currentTab === 'notifications'}
								<div class="space-y-6">
									<div>
										<h2 class="text-xl font-semibold mb-2">Notification Settings</h2>
										<p class="text-muted-foreground text-sm">Configure how you receive alerts and notifications</p>
									</div>

									<div class="grid grid-cols-1 gap-4 max-h-[400px] p-2 overflow-y-auto">
										<Card>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable Email Notifier</div>
														<div class="text-sm text-muted-foreground">Receive alerts via email</div>
													</div>
													<Switch id="email-alerts" checked={emailEnabled} onCheckedChange={toggleEmailNotifications} />
												</div>
												{#if emailEnabled}
													<div class="grid grid-cols-2 gap-3">
														<div>
															<Label for="smtp-username">SMTP Username</Label>
															<Input id="smtp-username" bind:value={smtpUsername} placeholder="user@example.com" />
														</div>
														<div>
															<Label for="smtp-password">SMTP Password</Label>
															<Input id="smtp-password" type="password" bind:value={smtpPassword} placeholder="App password" />
														</div>
													</div>
													<div class="grid grid-cols-2 gap-3">
														<div>
															<Label for="smtp-host">SMTP Host</Label>
															<Input id="smtp-host" bind:value={smtpHost} placeholder="smtp.gmail.com" />
														</div>
														<div>
															<Label for="smtp-port">SMTP Port</Label>
															<Input id="smtp-port" type="number" bind:value={smtpPort} placeholder="587" />
														</div>
													</div>
													<div>
														<Label for="email-from">From Address</Label>
														<Input id="email-from" type="email" bind:value={emailFrom} placeholder="alerts@company.com" />
													</div>
													<div>
														<Label for="notification-email">To Address(es)</Label>
														<Input id="notification-email" type="email" bind:value={emailTo} placeholder="admin@company.com,team@company.com" />
														<p class="text-xs text-muted-foreground mt-1">Comma-separated email addresses</p>
													</div>
												{/if}
											</CardContent>
										</Card>

										<Card>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable Slack Notifier</div>
														<div class="text-sm text-muted-foreground">Send alerts to Slack</div>
													</div>
													<Switch id="slack-notifications" checked={slackEnabled} onCheckedChange={toggleSlackNotifications} />
												</div>
												{#if slackEnabled}
													<div>
														<Label for="slack-bot-name">Bot Name</Label>
														<Input id="slack-bot-name" bind:value={slackBotName} placeholder="Optional bot name" />
													</div>
													<div class="grid grid-cols-3 gap-2">
														<div>
															<Label for="slack-token-a">Token A</Label>
															<Input id="slack-token-a" bind:value={slackTokenA} placeholder="Token A" />
														</div>
														<div>
															<Label for="slack-token-b">Token B</Label>
															<Input id="slack-token-b" bind:value={slackTokenB} placeholder="Token B" />
														</div>
														<div>
															<Label for="slack-token-c">Token C</Label>
															<Input id="slack-token-c" bind:value={slackTokenC} placeholder="Token C" />
														</div>
													</div>
												{/if}
											</CardContent>
										</Card>

										<Card>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable Discord Notifier</div>
														<div class="text-sm text-muted-foreground">Send alerts to Discord</div>
													</div>
													<Switch id="discord-notifications" checked={discordEnabled} onCheckedChange={toggleDiscordNotifications} />
												</div>
												{#if discordEnabled}
													<div>
														<Label for="discord-token">Token</Label>
														<Input id="discord-token" bind:value={discordToken} placeholder="Bot token" />
													</div>
													<div>
														<Label for="discord-channel-id">Channel ID</Label>
														<Input id="discord-channel-id" bind:value={discordChannelId} placeholder="Channel ID" />
													</div>
													<div>
														<Label for="discord-bot-name">Bot Name</Label>
														<Input id="discord-bot-name" bind:value={discordBotName} placeholder="Optional bot name" />
													</div>
												{/if}
											</CardContent>
										</Card>

										<Card>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable Telegram Notifier</div>
														<div class="text-sm text-muted-foreground">Send alerts to Telegram</div>
													</div>
													<Switch id="telegram-notifications" checked={telegramEnabled}  onCheckedChange={toggleTelegramNotifications} />
												</div>
												{#if telegramEnabled}
													<div>
														<Label for="telegram-token">Token</Label>
														<Input id="telegram-token" bind:value={telegramToken} placeholder="Bot token" />
													</div>
													<div>
														<Label for="telegram-chats">Chat IDs</Label>
														<Input id="telegram-chats" bind:value={telegramChats} placeholder="Comma-separated chat IDs" />
													</div>
												{/if}
											</CardContent>
										</Card>
									</div>
									<div class="flex justify-end">
										<Button onclick={saveNotificationSettings}>Save Notification Settings</Button>
									</div>
								</div>

							{:else if currentTab === 'alerts'}
								<div class="space-y-6">
									<div>
										<h2 class="text-xl font-semibold mb-2">Alert Rule Settings</h2>
										<p class="text-muted-foreground text-sm">Configure default alert behavior and retention</p>
									</div>

									<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
										<Card>
											<CardHeader>
												<CardTitle>Default Alert Settings</CardTitle>
												<CardDescription>Set default values for new alert rules</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div>
													<Label for="default-severity">Default Severity Level</Label>
													<select id="default-severity" bind:value={defaultSeverity} class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50">
														<option value="low">Low</option>
														<option value="medium">Medium</option>
														<option value="high">High</option>
														<option value="critical">Critical</option>
													</select>
												</div>
												<div>
													<Label for="alert-cooldown">Alert Cooldown (minutes)</Label>
													<Input id="alert-cooldown" type="number" bind:value={alertCooldown} placeholder="5" />
													<p class="text-xs text-muted-foreground mt-1">Minimum time between duplicate alerts</p>
												</div>
											</CardContent>
										</Card>

										<Card>
											<CardHeader>
												<CardTitle>Alert History</CardTitle>
												<CardDescription>Configure alert history and retention</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable Alert History</div>
														<div class="text-sm text-muted-foreground">Keep a record of all triggered alerts</div>
													</div>
													<Switch bind:checked={enableAlertHistory} />
												</div>
												<div>
													<Label for="alert-retention">Alert Retention (days)</Label>
													<Input id="alert-retention" type="number" bind:value={alertRetention} placeholder="30" />
													<p class="text-xs text-muted-foreground mt-1">How long to keep alert history</p>
												</div>
											</CardContent>
										</Card>
									</div>

									<div class="flex justify-end">
										<Button onclick={saveAlertSettings}>Save Alert Settings</Button>
									</div>
								</div>

							{:else if currentTab === 'systems'}
								<div class="space-y-6">
									<div>
										<h2 class="text-xl font-semibold mb-2">System Settings</h2>
										<p class="text-muted-foreground text-sm">Configure system monitoring and data retention</p>
									</div>

									<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
										<Card>
											<CardHeader>
												<CardTitle>System Monitoring</CardTitle>
												<CardDescription>Configure how systems are monitored</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div>
													<Label for="system-timeout">System Timeout (seconds)</Label>
													<Input id="system-timeout" type="number" bind:value={systemTimeout} placeholder="30" />
													<p class="text-xs text-muted-foreground mt-1">Time before marking a system as offline</p>
												</div>
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable System Logging</div>
														<div class="text-sm text-muted-foreground">Log system events and changes</div>
													</div>
													<Switch bind:checked={enableSystemLogging} />
												</div>
											</CardContent>
										</Card>

										<Card>
											<CardHeader>
												<CardTitle>Data Retention</CardTitle>
												<CardDescription>Configure how long to keep system data</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div>
													<Label for="metric-retention">Metric Retention (days)</Label>
													<Input id="metric-retention" type="number" bind:value={metricRetention} placeholder="90" />
													<p class="text-xs text-muted-foreground mt-1">How long to keep system metrics</p>
												</div>
												<div>
													<Label for="max-systems">Max Systems Per User</Label>
													<Input id="max-systems" type="number" bind:value={maxSystemsPerUser} placeholder="50" />
													<p class="text-xs text-muted-foreground mt-1">Maximum number of systems per user</p>
												</div>
											</CardContent>
										</Card>
									</div>

									<div class="flex justify-end">
										<Button onclick={saveSystemSettings}>Save System Settings</Button>
									</div>
								</div>

							{:else if currentTab === 'dashboard'}
								<div class="space-y-6">
									<div>
										<h2 class="text-xl font-semibold mb-2">Dashboard Settings</h2>
										<p class="text-muted-foreground text-sm">Customize your dashboard experience</p>
									</div>

									<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
										<Card>
											<CardHeader>
												<CardTitle>Display Settings</CardTitle>
												<CardDescription>Configure dashboard appearance</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Dark Mode</div>
														<div class="text-sm text-muted-foreground">Use dark theme</div>
													</div>
													<Switch bind:checked={darkMode} />
												</div>
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Compact View</div>
														<div class="text-sm text-muted-foreground">Use compact dashboard layout</div>
													</div>
													<Switch bind:checked={compactView} />
												</div>
											</CardContent>
										</Card>

										<Card>
											<CardHeader>
												<CardTitle>Refresh Settings</CardTitle>
												<CardDescription>Configure data refresh behavior</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Auto Refresh</div>
														<div class="text-sm text-muted-foreground">Automatically refresh dashboard data</div>
													</div>
													<Switch bind:checked={enableAutoRefresh} />
												</div>
												<div>
													<Label for="refresh-rate">Default Refresh Rate (seconds)</Label>
													<Input id="refresh-rate" type="number" bind:value={defaultRefreshRate} placeholder="30" />
													<p class="text-xs text-muted-foreground mt-1">How often to refresh dashboard data</p>
												</div>
											</CardContent>
										</Card>
									</div>

									<div class="flex justify-end">
										<Button onclick={saveDashboardSettings}>Save Dashboard Settings</Button>
									</div>
								</div>

							{:else if currentTab === 'agents'}
								<div class="space-y-6">
									<div>
										<h2 class="text-xl font-semibold mb-2">Agent Settings</h2>
										<p class="text-muted-foreground text-sm">Configure monitoring agent behavior</p>
									</div>

									<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
										<Card>
											<CardHeader>
												<CardTitle>Agent Configuration</CardTitle>
												<CardDescription>Configure how agents communicate and behave</CardDescription>
											</CardHeader>
											<CardContent class="space-y-4">
												<div>
													<Label for="agent-update-interval">Update Interval (seconds)</Label>
													<Input id="agent-update-interval" type="number" bind:value={agentUpdateInterval} placeholder="60" />
													<p class="text-xs text-muted-foreground mt-1">How often agents send updates</p>
												</div>
												<div>
													<Label for="max-connections">Max Agent Connections</Label>
													<Input id="max-connections" type="number" bind:value={maxAgentConnections} placeholder="100" />
													<p class="text-xs text-muted-foreground mt-1">Maximum concurrent agent connections</p>
												</div>
												<div class="flex items-center justify-between">
													<div>
														<div class="font-medium">Enable Agent Logging</div>
														<div class="text-sm text-muted-foreground">Log agent communication and events</div>
													</div>
													<Switch bind:checked={enableAgentLogging} />
												</div>
											</CardContent>
										</Card>

										<Card>
											<CardHeader>
												<CardTitle>Agent Status</CardTitle>
												<CardDescription>View connected agents and their status</CardDescription>
											</CardHeader>
											<CardContent>
												<div class="space-y-3">
													<div class="flex justify-between items-center p-3 border border-border rounded-lg">
														<div class="flex items-center gap-3">
															<span class="w-2 h-2 bg-green-500 rounded-full"></span>
															<div>
																<div class="font-medium">web-server-01</div>
																<div class="text-sm text-muted-foreground">192.168.1.10</div>
															</div>
														</div>
														<div class="text-sm text-muted-foreground">Connected</div>
													</div>
													<div class="flex justify-between items-center p-3 border border-border rounded-lg">
														<div class="flex items-center gap-3">
															<span class="w-2 h-2 bg-green-500 rounded-full"></span>
															<div>
																<div class="font-medium">db-server-01</div>
																<div class="text-sm text-muted-foreground">192.168.1.20</div>
															</div>
														</div>
														<div class="text-sm text-muted-foreground">Connected</div>
													</div>
													<div class="flex justify-between items-center p-3 border border-border rounded-lg">
														<div class="flex items-center gap-3">
															<span class="w-2 h-2 bg-red-500 rounded-full"></span>
															<div>
																<div class="font-medium">backup-server-01</div>
																<div class="text-sm text-muted-foreground">192.168.1.30</div>
															</div>
														</div>
														<div class="text-sm text-muted-foreground">Disconnected</div>
													</div>
												</div>
											</CardContent>
										</Card>
									</div>

									<div class="flex justify-end">
										<Button onclick={saveAgentSettings}>Save Agent Settings</Button>
									</div>
								</div>
							{/if}
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</div>

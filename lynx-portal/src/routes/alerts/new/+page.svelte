<script lang="ts">

	import { Switch } from '$lib/components/ui/switch/index';
	import { Button } from '$lib/components/ui/button/index';
	import { Input } from '$lib/components/ui/input/index';
	import * as Select from '$lib/components/ui/select/index';
	import { Label } from '$lib/components/ui/label';
	import {enhance} from '$app/forms';
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner'



	type Expression = {
		id: number;
		field: string;
		operator: string;
		value: string;
		next_operator?: string; // Optional for chaining expressions
	}

	let rules = $state([] as Expression[]);
	let fieldValue = $state('');
	let operatorValue = $state('');
	let valueValue = $state('');
	let editor =  $state('builder');
	let ruleName = $state('');
	let ruleDescription = $state('');
	let valid = $state(false);
	let rawExpression = $derived.by(() => {
		return rules.length > 0 ? rules.map(rule => `${rule.field} ${rule.operator} ${rule.value} ${rule.next_operator ?? 'OR'}`).join(' ') : '';
	});

	$effect(() => {
		valid = !!rawExpression.length;
	});

	function sendForm() {
		// submit post form
		fetch('/alerts/new', {
			method: 'POST',
			body: JSON.stringify({
				name: ruleName,
				description: ruleDescription,
				severity: 'low',
				expression: rawExpression
			}),
			headers: {
				'Content-Type': 'application/json'
			}
		}).then(response => {
			if (response.ok) {
				toast.success(`Created new alert rule: ${ruleName}`);
				window.location.href = "/alerts";
			} else {
				toast.error(`Failed to create alert rule: ${ruleName}`);
			}
		}).catch(error => {
			toast.error('Error creating alert rule', {
				description: error ?? "Unknown error occurred"
			})
		})
	}

</script>

<div class="">
	<div class="flex flex-col items-start gap-2 mt-4 w-full">
		<h1 class="text-2xl font-bold">Create Rule</h1>
		<p class="text-sm text-muted-foreground">Build expressions to create complex alert rules.</p>
		<Switch checked={true} />
		<span class="text-sm text-muted-foreground">Enable Rule</span>
	</div>

	<div class="flex items-center gap-2 mt-4 w-full">
		<Input autocomplete={null} aria-autocomplete="none" data-lpignore="true" type="text" placeholder="Rule Title" class="input input-bordered w-64" bind:value={ruleName} />
		<Input autocomplete={null} aria-autocomplete="none" data-lpignore="true" type="text" placeholder="Description" class="input input-bordered w-64" bind:value={ruleDescription} />
	</div>

	<div class="mt-4 flex flex-col max-w-[500px]">
		<p class="text-sm text-muted-foreground mb-5">
			Add conditions to your rule:
			<span class={`cursor-pointer ${editor == "builder" ? 'text-primary' : 'text-muted-foreground'} transition-colors`} onclick={() => editor = "builder"}>Expression builder</span>
			{editor == "builder" ? ' / ' : ' \\ '}
			<span class={`cursor-pointer ${editor == "raw" ? 'text-primary' : 'text-muted-foreground'} transition-colors`} onclick={() => editor = "raw"}>Raw expression</span>
		</p>

		{#if editor === 'raw'}
			<textarea
				class="textarea textarea-bordered bg-[var(--foreground)] border border-border  text-sm font-mono outline-none w-full h-auto p-2"
				bind:value={rawExpression}
				onblur={() => {
					if (!rawExpression.trim()) {
						rules = [];
						return;
					}
					try {

						const nextOperators = rawExpression.match(/ AND | OR /g) || [];

					rules = rawExpression.split(/ AND | OR /).map((expr, index) => {
						const parts = expr.split(' ');
						// check if parts are valid
						if (parts.length < 3) {
							throw new Error('Invalid expression format');
						}
						// Ensure the operator is valid
						const validOperators = ['=', '!=', '>', '<'];
						if (!validOperators.includes(parts[1])) {
							throw new Error(`Invalid operator: ${parts[1]}`);
						}
						// find current AND/OR operator in original expression at the same index
						const next_operator = nextOperators[index] ? nextOperators[index].trim() : '';
						return {
							id: index + 1,
							field: parts[0],
							operator: parts[1],
							value: parts.slice(2).join(' '),
							next_operator:  next_operator
						};
					});
					} catch (error) {
						console.error('Error parsing expression:', error);
						valid = false;
						alert('Invalid expression format. Please check your input.');
					}
				}}
			>
			</textarea>
		{/if}
		{#if editor === 'builder'}
			{#each rules as rule}
				<div class="flex items-center justify-between  rounded-lg mb-2" id="rule-{rule.id.toString()}">
					<div class="flex items-center gap-2">
						<Input disabled type="text" bind:value={rule.field} placeholder="Field" class="input input-bordered w-32" />
						<Select.Root disabled type="single" bind:value={rule.operator}>
							<Select.Trigger class="w-[180px] flex items-center align-middle gap-0">
						<span class="flex items-center gap-2">
							<span class="text-sm">{rule.operator}</span>
						</span>
							</Select.Trigger>
							<Select.Content class="bg-[var(--background)] rounded-md border border-[var(--border)]">
								<Select.Item value="=">=</Select.Item>
								<Select.Item value="!=">!=</Select.Item>
								<Select.Item value=">">{">"}</Select.Item>
								<Select.Item value="<">{"<"}</Select.Item>
							</Select.Content>
						</Select.Root>
						<Input disabled type="text" bind:value={rule.value} placeholder="Value" class="input input-bordered w-32" />
						<Button variant="ghost" class="cursor-pointer" size="icon" onclick={() => {
					rules = rules.filter(r => r.id !== rule.id);
				}}>
							<span class="icon-[line-md--trash] w-5 h-5"></span>
						</Button>
					</div>
				</div>
				<div class={`flex items-center justify-between ${rule.next_operator === 'AND' ? 'mb-2' : 'mt-5 mb-5'}`}>
				<span class="text-sm text-muted-foreground">
					{rule.next_operator}
				</span>
				</div>
			{/each}
			<div class="flex items-center mb-2 gap-1">
				<div class="flex items-center gap-2">
					<Select.Root type="single" bind:value={fieldValue} onValueChange={(val) => {
					fieldValue = val;
				}}>
						<Select.Trigger class="w-[180px] flex items-center align-middle gap-0">
						<span class="flex items-center gap-2">
							<span class="icon-[line-md--filter] w-4 h-4"></span>
							<span class="text-sm">{fieldValue || 'Select Field'}</span>
						</span>
						</Select.Trigger>
						<Select.Content class="bg-[var(--background)] rounded-md border border-[var(--border)]">
							<Select.Group>
								<Select.Label>CPU</Select.Label>
								<Select.Item value="cpu.usage">CPU Usage (%)</Select.Item>
							</Select.Group>
							<Select.Group>
								<Select.Label>Loads</Select.Label>
								<Select.Item value="load.one">Load 1</Select.Item>
								<Select.Item value="load.five">Load 5</Select.Item>
								<Select.Item value="load.fifteen">Load 15</Select.Item>
							</Select.Group>
							<Select.Group>
								<Select.Label>Memory</Select.Label>
								<Select.Item value="memory.usage">Memory Usage (%)</Select.Item>
							</Select.Group>
							<Select.Group>
								<Select.Label>Network</Select.Label>
								<Select.Item value="network.in">Network In (mb/s)</Select.Item>
								<Select.Item value="network.out">Network Out (mb/s)</Select.Item>
							</Select.Group>
							<Select.Group>
								<Select.Label>Disk</Select.Label>
								<Select.Item value="disk.usage">Disk Usage (%)</Select.Item>
								<Select.Item value="disk.read">Disk Read (mb/s)</Select.Item>
								<Select.Item value="disk.write">Disk Write (mb/s)</Select.Item>
							</Select.Group>
							<Select.Group>
								<Select.Label>Temperature</Select.Label>
								<Select.Item value="temp">Temperature (°C)</Select.Item>
							</Select.Group>
						</Select.Content>
					</Select.Root>

					<Select.Root type="single" bind:value={operatorValue}>
						<Select.Trigger class="w-[180px] flex items-center align-middle gap-0">
						<span class="flex items-center gap-2">
							<span class="text-sm">{operatorValue || 'Select Operator'}</span>
						</span>
						</Select.Trigger>
						<Select.Content class="bg-[var(--background)] rounded-md border border-[var(--border)]">
							<Select.Item value="=">=</Select.Item>
							<Select.Item value="!=">!=</Select.Item>
							<Select.Item value=">">{">"}</Select.Item>
							<Select.Item value="<">{"<"}</Select.Item>
						</Select.Content>
					</Select.Root>
					<Input type="text" placeholder="Value" bind:value={valueValue} class="input input-bordered w-32" />
				</div>
				<Button variant="ghost" size="icon" onclick={() => {
					const newRule: Expression = {
						id: rules.length + 1,
						field: fieldValue || '',
						operator: operatorValue || '',
						value: valueValue || '',
						next_operator: 'AND', // Default next operator
					};
					rules.push(newRule);
					fieldValue = '';
					operatorValue = '';
					valueValue = '';
					// check previous rule's next_operator
					if (rules.length > 1) {
						const lastRule = rules[rules.length - 2];
						if (lastRule.next_operator === '') {
							lastRule.next_operator = 'OR'; // Change last rule's next operator to AND
						}
					}
			}}>
					AND
				</Button>
				<Button variant="ghost" size="icon" onclick={() => {
				const newRule: Expression = {
					id: rules.length + 1,
					field: fieldValue || '',
					operator: operatorValue || '',
					value: valueValue || '',
					next_operator: 'OR', // Default next operator
				};
				rules.push(newRule);
				fieldValue = '';
				operatorValue = '';
				valueValue = '';
						if (rules.length > 1) {
						const lastRule = rules[rules.length - 2];
						if (lastRule.next_operator === '') {
							lastRule.next_operator = 'OR'; // Change last rule's next operator to AND
						}
					}
			}}>
					OR
				</Button>
			</div>
		{/if}
	</div>

	<div class="mt-4 flex flex-col">
		<h1 class="text-lg font-extrabold">
			Notifiers
		</h1>
		<p class="text-sm text-muted-foreground mb-5">Select notifiers to use for this rule.</p>

		<div class="space-y-2 w-1/2">
			<div class="flex items-center justify-between w-full gap-2">
				<Label for="email-notifier">Emails</Label>
				<Switch id="email-notifier" class="cursor-pointer" checked={true} />
			</div>
			<div class="flex items-center  justify-between w-full gap-2">
				<Label for="webhook-notifier">Discord</Label>
				<Switch id="webhook-notifier" class="cursor-pointer" checked={false} />
			</div>
		</div>
	</div>

	<div class="mt-4">
		<Button  disabled={!valid} class="bg-primary/50 border border-primary cursor-pointer active:scale-95" onclick={() => {
			sendForm();
			// Reset form after submission
			rules = [];
			fieldValue = '';
			operatorValue = '';
			valueValue = '';
			rawExpression = '';
			editor = 'builder';
		}}>
			Create Rule
		</Button>
	</div>
</div>
<script lang="ts">

	import { Switch } from '$lib/components/ui/switch/index';
	import { Button } from '$lib/components/ui/button/index';
	import { Input } from '$lib/components/ui/input/index';
	import * as Select from '$lib/components/ui/select/index';
	import { Label } from '$lib/components/ui/label';

	type Expression = {
		id: number;
		field: string;
		operator: string;
		value: string;
		next_operator?: string; // Optional for chaining expressions
	}

	let rules = $state([
		{
			id: 1,
			field: 'cpu',
			operator: '>',
			value: '80',
			next_operator: 'AND'
		},
		{
			id: 2,
			field: 'memory',
			operator: '<',
			value: '20',
			next_operator: 'OR'
		}
	] as Expression[]);
	let fieldValue = $state('');
	let operatorValue = $state('');
	let valueValue = $state('');

</script>

<div>
	<h1 class="text-2xl font-bold">Create Rule</h1>
	<p class="text-sm text-muted-foreground">Build expressions to create complex alert rules.</p>

	<div class="flex items-center gap-2 mt-4">
		<Switch checked={true} />
		<span class="text-sm text-muted-foreground">Enable Rule</span>
	</div>

	<div class="flex items-center gap-2 mt-4">
		<Input type="text" placeholder="Rule Name" class="input input-bordered w-64" />
		<Input type="text" placeholder="Description" class="input input-bordered w-64" />
	</div>

	<div class="mt-4 flex flex-col">
		<p class="text-sm text-muted-foreground mb-5">Add conditions to your rule:</p>
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
		<div class="flex items-center justify-between mb-2 gap-1">
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
						<Select.Item value="metric">Metric Name</Select.Item>
						<Select.Item value="cpu">CPU Usage (%)</Select.Item>
						<Select.Item value="memory">Memory Usage (%)</Select.Item>
						<Select.Item value="l1">Load 1</Select.Item>
						<Select.Item value="l5">Load 5</Select.Item>
						<Select.Item value="l15">Load 15</Select.Item>
						<Select.Item value="disk">Disk Usage (%)</Select.Item>
						<Select.Item value="in">  In (mb/s)</Select.Item>
						<Select.Item value="out">Network Out (mb/s)</Select.Item>
						<Select.Item value="temp">Temperature (°C)</Select.Item>
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
			}}>
				OR
			</Button>
		</div>
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
				<Label for="webhook-notifier">Webhook</Label>
				<Switch id="webhook-notifier" class="cursor-pointer" checked={false} />
			</div>
		</div>
	</div>

	<div class="mt-4">
		<Button class="bg-primary/50 border border-primary cursor-pointer active:scale-95" onclick={() => {
			alert('Rule created!'); // Replace with actual rule creation logic
		}}>
			Create Rule
		</Button>
	</div>
</div>
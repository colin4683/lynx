<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { WidgetType } from './types';

	interface Props {
		isVisible: boolean;
	}

	let { isVisible = false }: Props = $props();

	const dispatch = createEventDispatcher();

	const availableWidgets = [
		{
			type: 'chart' as WidgetType,
			title: 'Chart Widget',
			description: 'Display data in charts',
			icon: '📊',
			defaultSize: { width: 3, height: 2 }
		},
		{
			type: 'table' as WidgetType,
			title: 'Table Widget',
			description: 'Show data in tables',
			icon: '📋',
			defaultSize: { width: 3, height: 3 }
		},
		{
			type: 'notification' as WidgetType,
			title: 'Notifications',
			description: 'Display alerts and notifications',
			icon: '🔔',
			defaultSize: { width: 2, height: 3 }
		},
		{
			type: 'metric' as WidgetType,
			title: 'Metric Display',
			description: 'Show single metrics',
			icon: '📈',
			defaultSize: { width: 2, height: 2 }
		},
		{
			type: 'progress' as WidgetType,
			title: 'Progress Bars',
			description: 'Display progress indicators',
			icon: '📊',
			defaultSize: { width: 2, height: 2 }
		}
	];

	function handleDragStart(event: DragEvent, widgetType: WidgetType, defaultSize: { width: number; height: number }) {
		const widgetData = {
			type: widgetType,
			defaultSize
		};
		event.dataTransfer!.setData('text/plain', JSON.stringify(widgetData));
		event.dataTransfer!.effectAllowed = 'copy';
	}

	function addWidget(widgetType: WidgetType, defaultSize: { width: number; height: number }) {
		dispatch('addWidget', {
			type: widgetType,
			defaultSize
		});
	}
</script>

<div
	class="widget-sidebar fixed right-0 top-0 h-full bg-background border-l border-border shadow-lg z-50 transition-transform duration-300"
	class:translate-x-0={isVisible}
	class:translate-x-full={!isVisible}
	style="width: 300px;"
>
	<div class="p-4 border-b border-border">
		<h3 class="text-lg font-semibold">Available Widgets</h3>
		<p class="text-sm text-muted-foreground">Drag widgets to the dashboard or click to add</p>
	</div>

	<div class="p-4 space-y-3 overflow-y-auto h-full pb-20">
		{#each availableWidgets as widget}
			<button
				class="widget-item w-full text-left p-3 border border-border rounded-lg cursor-grab hover:bg-accent transition-colors"
				draggable="true"
				role="button"
				tabindex="0"
				ondragstart={(e) => handleDragStart(e, widget.type, widget.defaultSize)}
				onclick={() => addWidget(widget.type, widget.defaultSize)}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						addWidget(widget.type, widget.defaultSize);
					}
				}}
			>
				<div class="flex items-start gap-3">
					<div class="text-2xl">{widget.icon}</div>
					<div class="flex-1">
						<h4 class="font-medium text-sm">{widget.title}</h4>
						<p class="text-xs text-muted-foreground mt-1">{widget.description}</p>
						<div class="text-xs text-muted-foreground mt-2">
							Default size: {widget.defaultSize.width}×{widget.defaultSize.height}
						</div>
					</div>
				</div>
			</button>
		{/each}
	</div>
</div>

<style>
	.widget-sidebar {
		backdrop-filter: blur(8px);
	}

	.widget-item:active {
		cursor: grabbing;
	}
</style>

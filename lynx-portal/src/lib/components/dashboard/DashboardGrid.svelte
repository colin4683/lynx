<script lang="ts">
	import type { Widget } from './types.js';

	interface Props {
		widgets: Widget[];
		isEditMode: boolean;
		gridCols?: number;
		gridRows?: number;
	}

	let { widgets = $bindable(), isEditMode = false, gridCols = 12, gridRows = 8 }: Props = $props();

	let draggedWidget: Widget | null = null;
	let draggedElement: HTMLElement | null = null;
	let gridElement: HTMLElement;

	function handleDragStart(event: DragEvent, widget: Widget) {
		if (!isEditMode) return;
		draggedWidget = widget;
		draggedElement = event.target as HTMLElement;
		event.dataTransfer!.effectAllowed = 'move';
		event.dataTransfer!.setData('text/plain', 'existing-widget');
	}

	function handleDragOver(event: DragEvent) {
		if (!isEditMode) return;
		event.preventDefault();
		event.dataTransfer!.dropEffect = 'move';
	}

	function handleDrop(event: DragEvent) {
		if (!isEditMode) return;
		event.preventDefault();

		const rect = gridElement.getBoundingClientRect();
		const x = event.clientX - rect.left;
		const y = event.clientY - rect.top;

		const cellWidth = rect.width / gridCols;
		const cellHeight = rect.height / gridRows;

		const gridX = Math.floor(x / cellWidth);
		const gridY = Math.floor(y / cellHeight);

		// Check if this is a new widget from sidebar
		const widgetData = event.dataTransfer?.getData('text/plain');
		if (widgetData && !draggedWidget) {
			try {
				const { type, defaultSize } = JSON.parse(widgetData);
				const newWidget: Widget = {
					id: crypto.randomUUID(),
					type,
					title: `${type.charAt(0).toUpperCase() + type.slice(1)} Widget`,
					x: Math.max(0, Math.min(gridX, gridCols - defaultSize.width)),
					y: Math.max(0, Math.min(gridY, gridRows - defaultSize.height)),
					width: defaultSize.width,
					height: defaultSize.height
				};
				widgets = [...widgets, newWidget];
				return;
			} catch (e) {
				console.error('Error parsing widget data:', e);
			}
		}

		// Handle existing widget movement
		if (draggedWidget) {
			widgets = widgets.map(w =>
				w.id === draggedWidget!.id
					? { ...w, x: Math.max(0, Math.min(gridX, gridCols - w.width)), y: Math.max(0, Math.min(gridY, gridRows - w.height)) }
					: w
			);

			draggedWidget = null;
			draggedElement = null;
		}
	}

	function removeWidget(widgetId: string) {
		widgets = widgets.filter(w => w.id !== widgetId);
	}

	function resizeWidget(widget: Widget, newWidth: number, newHeight: number) {
		widgets = widgets.map(w =>
			w.id === widget.id
				? { ...w, width: Math.max(1, Math.min(newWidth, gridCols - w.x)), height: Math.max(1, Math.min(newHeight, gridRows - w.y)) }
				: w
		);
	}

	function getGridPosition(widget: Widget) {
		return {
			gridColumn: `${widget.x + 1} / span ${widget.width}`,
			gridRow: `${widget.y + 1} / span ${widget.height}`
		};
	}
</script>

<div
	bind:this={gridElement}
	class="dashboard-grid relative w-full h-full border-2 border-dashed border-transparent"
	class:border-primary={isEditMode}
	style="display: grid; grid-template-columns: repeat({gridCols}, 1fr); grid-template-rows: repeat({gridRows}, 1fr); gap: 8px; min-height: 600px;"
	role="application"
	aria-label="Dashboard grid for arranging widgets"
	ondragover={handleDragOver}
	ondrop={handleDrop}
>
	{#each widgets as widget (widget.id)}
		<div
			class="widget-container relative bg-background border border-border rounded-lg shadow-sm"
			class:cursor-move={isEditMode}
			style={`${Object.entries(getGridPosition(widget)).map(([key, value]) => `${key}: ${value}`).join('; ')}`}
			draggable={isEditMode}
			role="button"
			tabindex={isEditMode ? 0 : -1}
			aria-label="Widget: {widget.title}"
			ondragstart={(e) => handleDragStart(e, widget)}
		>
			{#if isEditMode}
				<div class="absolute top-1 right-1 flex gap-1 z-10">
					<select
						class="text-xs px-1 py-0.5 bg-background border border-border rounded"
						value={`${widget.width}x${widget.height}`}
						onchange={(e) => {
							const [w, h] = e.currentTarget.value.split('x').map(Number);
							resizeWidget(widget, w, h);
						}}
					>
						<option value="1x1">1x1</option>
						<option value="2x1">2x1</option>
						<option value="1x2">1x2</option>
						<option value="2x2">2x2</option>
						<option value="3x2">3x2</option>
						<option value="2x3">2x3</option>
						<option value="3x3">3x3</option>
						<option value="4x2">4x2</option>
						<option value="2x4">2x4</option>
						<option value="4x3">4x3</option>
						<option value="3x4">3x4</option>
					</select>
					<button
						class="text-xs px-1 py-0.5 bg-red-500 text-white rounded hover:bg-red-600"
						onclick={() => removeWidget(widget.id)}
					>
						×
					</button>
				</div>
			{/if}

			<div class="p-3 h-full">
				{#if widget.type === 'chart'}
					<div class="h-full flex flex-col">
						<h3 class="text-sm font-semibold mb-2">{widget.title}</h3>
						<div class="flex-1 bg-gradient-to-br from-blue-50 to-blue-100 rounded border flex items-center justify-center">
							<span class="text-blue-600 text-xs">Chart Widget</span>
						</div>
					</div>
				{:else if widget.type === 'table'}
					<div class="h-full flex flex-col">
						<h3 class="text-sm font-semibold mb-2">{widget.title}</h3>
						<div class="flex-1 overflow-auto">
							<table class="w-full text-xs">
								<thead>
									<tr class="border-b">
										<th class="text-left p-1">Name</th>
										<th class="text-left p-1">Status</th>
									</tr>
								</thead>
								<tbody>
									<tr><td class="p-1">Server 1</td><td class="p-1 text-green-600">Online</td></tr>
									<tr><td class="p-1">Server 2</td><td class="p-1 text-red-600">Offline</td></tr>
									<tr><td class="p-1">Server 3</td><td class="p-1 text-green-600">Online</td></tr>
								</tbody>
							</table>
						</div>
					</div>
				{:else if widget.type === 'notification'}
					<div class="h-full flex flex-col">
						<h3 class="text-sm font-semibold mb-2">{widget.title}</h3>
						<div class="flex-1 space-y-2 overflow-auto">
							<div class="p-2 bg-red-50 border border-red-200 rounded text-xs">
								<div class="flex items-center gap-1">
									<span class="w-2 h-2 bg-red-500 rounded-full"></span>
									<span class="font-medium">High CPU Usage</span>
								</div>
								<p class="text-muted-foreground mt-1">Server overloaded</p>
							</div>
							<div class="p-2 bg-yellow-50 border border-yellow-200 rounded text-xs">
								<div class="flex items-center gap-1">
									<span class="w-2 h-2 bg-yellow-500 rounded-full"></span>
									<span class="font-medium">Low Disk Space</span>
								</div>
								<p class="text-muted-foreground mt-1">Storage running low</p>
							</div>
						</div>
					</div>
				{:else if widget.type === 'metric'}
					<div class="h-full flex flex-col">
						<h3 class="text-sm font-semibold mb-2">{widget.title}</h3>
						<div class="flex-1 flex items-center justify-center">
							<div class="text-center">
								<div class="text-2xl font-bold text-primary">24.7%</div>
								<div class="text-xs text-muted-foreground">CPU Usage</div>
							</div>
						</div>
					</div>
				{:else if widget.type === 'progress'}
					<div class="h-full flex flex-col">
						<h3 class="text-sm font-semibold mb-2">{widget.title}</h3>
						<div class="flex-1 space-y-3">
							<div>
								<div class="flex justify-between text-xs mb-1">
									<span>Memory</span>
									<span>68%</span>
								</div>
								<div class="w-full bg-gray-200 rounded-full h-2">
									<div class="bg-blue-600 h-2 rounded-full" style="width: 68%"></div>
								</div>
							</div>
							<div>
								<div class="flex justify-between text-xs mb-1">
									<span>Storage</span>
									<span>42%</span>
								</div>
								<div class="w-full bg-gray-200 rounded-full h-2">
									<div class="bg-green-600 h-2 rounded-full" style="width: 42%"></div>
								</div>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/each}
</div>

<style>
	.dashboard-grid {
		transition: border-color 0.2s ease;
	}

	.widget-container {
		transition: transform 0.2s ease, box-shadow 0.2s ease;
	}

	.widget-container:hover {
		transform: translateY(-1px);
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
	}
</style>

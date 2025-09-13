<script lang="ts">
	import { getStatusConfig, type SystemStatus } from '$lib/utils/statusConfig';

	const { status, size = 'sm', variant = 'default' }: {
		status: SystemStatus;
		size?: 'xs' | 'sm' | 'md' | 'lg';
		variant?: 'default' | 'compact' | 'dot-only';
	} = $props();

	const config = $derived(getStatusConfig(status));

	const sizeClasses = {
		xs: 'px-2 py-1 text-xs',
		sm: 'px-2.5 py-1 text-sm',
		md: 'px-3 py-1.5 text-sm',
		lg: 'px-4 py-2 text-base'
	};

	const dotSizes = {
		xs: 'w-1.5 h-1.5',
		sm: 'w-2 h-2',
		md: 'w-2.5 h-2.5',
		lg: 'w-3 h-3'
	};

	const iconSizes = {
		xs: 'w-3 h-3',
		sm: 'w-4 h-4',
		md: 'w-5 h-5',
		lg: 'w-6 h-6'
	};
</script>

{#if variant === 'dot-only'}
	<div
		class="rounded-full {dotSizes[size]} {config.colors.dot}"
		title={config.label}
	></div>
{:else if variant === 'compact'}
	<div class="flex items-center gap-1.5">
		<div class="rounded-full {dotSizes[size]} {config.colors.dot}"></div>
		<span class="{config.colors.text} text-{size} font-medium">{config.label}</span>
	</div>
{:else}
	<div class="inline-flex items-center gap-2 {sizeClasses[size]} rounded-md border {config.colors.bg} {config.colors.text} {config.colors.border} transition-colors {config.colors.hover}">
		<span class="{config.icon} {iconSizes[size]}"></span>
		<span class="font-medium">{config.label}</span>
	</div>
{/if}

export type SystemStatus = 'online' | 'offline' | 'error' | 'warning' | 'maintenance' | 'unknown';

export interface StatusConfig {
	label: string;
	icon: string;
	colors: {
		bg: string;
		text: string;
		border: string;
		dot: string;
		hover: string;
	};
	priority: number; // Higher number = more critical
}

// Centralized status configuration - easy to modify colors and add new statuses
export const STATUS_CONFIGS: Record<SystemStatus, StatusConfig> = {
	online: {
		label: 'Online',
		icon: 'icon-[heroicons--check-circle]',
		colors: {
			bg: 'bg-[var(--primary)]/20',
			text: 'text-[var(--primary)]',
			border: 'border-[var(--primary)]/30',
			dot: 'bg-[var(--primary)]',
			hover: 'hover:bg-[var(--primary)]/30'
		},
		priority: 1
	},
	warning: {
		label: 'Warning',
		icon: 'icon-[heroicons--exclamation-triangle]',
		colors: {
			bg: 'bg-[var(--disk)]/20',
			text: 'text-[var(--disk)]',
			border: 'border-[var(--disk)]/30',
			dot: 'bg-[var(--disk)]',
			hover: 'hover:bg-[var(--disk)]/30'
		},
		priority: 3
	},
	error: {
		label: 'Error',
		icon: 'icon-[heroicons--x-circle]',
		colors: {
			bg: 'bg-[var(--memory)]/20',
			text: 'text-[var(--memory)]',
			border: 'border-[var(--memory)]/30',
			dot: 'bg-[var(--memory)]',
			hover: 'hover:bg-[var(--memory)]/30'
		},
		priority: 4
	},
	offline: {
		label: 'Offline',
		icon: 'icon-[heroicons--signal-slash]',
		colors: {
			bg: 'bg-[var(--text)]/10',
			text: 'text-[var(--text)]/80',
			border: 'border-[var(--border)]',
			dot: 'bg-[var(--text)]/60',
			hover: 'hover:bg-[var(--text)]/20'
		},
		priority: 2
	},
	maintenance: {
		label: 'Maintenance',
		icon: 'icon-[heroicons--wrench-screwdriver]',
		colors: {
			bg: 'bg-[var(--cpu)]/20',
			text: 'text-[var(--cpu)]',
			border: 'border-[var(--cpu)]/30',
			dot: 'bg-[var(--cpu)]',
			hover: 'hover:bg-[var(--cpu)]/30'
		},
		priority: 1
	},
	unknown: {
		label: 'Unknown',
		icon: 'icon-[heroicons--question-mark-circle]',
		colors: {
			bg: 'bg-[var(--border)]/20',
			text: 'text-[var(--text)]/60',
			border: 'border-[var(--border)]',
			dot: 'bg-[var(--text)]/40',
			hover: 'hover:bg-[var(--border)]/30'
		},
		priority: 0
	}
};

// Helper function to get status config
export function getStatusConfig(status: SystemStatus): StatusConfig {
	return STATUS_CONFIGS[status] || STATUS_CONFIGS.unknown;
}

// Helper function to get status priority for sorting
export function getStatusPriority(status: SystemStatus): number {
	return STATUS_CONFIGS[status]?.priority || 0;
}

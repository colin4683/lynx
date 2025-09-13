export type SystemStatus = 'online' | 'offline' | 'error' | 'warning';

export interface System {
	id: number;
	label: string;
	hostname?: string | null;
	active: boolean | null;
	lastSeen?: string | null;
	cpuUsage?: number | null;
	memoryTotal?: number | null;
	memoryUsed?: number | null;
	disks?: Array<{
		used?: number | null;
		space?: number | null;
	}>;
	metrics?: Array<{
		memoryTotalKb?: number | null;
		memoryUsedKb?: number | null;
	}>;
	alertHistories?: Array<{
		date: string;
	}>;
}

/**
 * Determines the status of a system based on its properties
 */
export function getSystemStatus(system: System): SystemStatus {
	// If system is not active, it's offline
	if (!system.active) return 'offline';

	// Check if system hasn't been seen recently (error state)
	if (system.lastSeen) {
		const lastSeenDate = new Date(system.lastSeen);
		const now = new Date();
		const diffMinutes = (now.getTime() - lastSeenDate.getTime()) / (1000 * 60);

		// If not seen in over 10 minutes, consider it an error
		if (diffMinutes > 10) return 'error';

		// If not seen in over 5 minutes, consider it a warning
		if (diffMinutes > 5) return 'warning';
	}

	// Check for high resource usage that might indicate issues
	if (system.alertHistories && system.alertHistories.length > 0) return 'warning';
	// If all checks pass, system is online
	return 'online';
}

/**
 * Gets a metric value for display purposes
 */
export function getMetricValue(system: System, metric: 'cpu' | 'memory' | 'disk'): number {
	switch (metric) {
		case 'cpu':
			return system.cpuUsage || 0;
		case 'memory':
			if (system.memoryTotal && system.memoryUsed) {
				return (system.memoryUsed / system.memoryTotal) * 100;
			}
			// Fallback to metrics data
			if (system.metrics?.[0]?.memoryTotalKb && system.metrics?.[0]?.memoryUsedKb) {
				return (system.metrics[0].memoryUsedKb / system.metrics[0].memoryTotalKb) * 100;
			}
			return 0;
		case 'disk':
			if (system.disks?.[0]?.used && system.disks?.[0]?.space) {
				return (system.disks[0].used / system.disks[0].space) * 100;
			}
			return 0;
	}
}

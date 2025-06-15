import { db } from '$lib/server/db';
import { error } from '@sveltejs/kit';
import { and, eq, gte, sql, sum } from 'drizzle-orm';
import { interval } from 'drizzle-orm/pg-core';

function parseTimeRangeToMs(range: string): number {
	const [value, unit] = range.split(' ');
	const num = parseInt(value);

	switch (unit) {
		case 'seconds':
		case 'second':
			return num * 1000;
		case 'minutes':
		case 'minute':
			return num * 60 * 1000;
		case 'hours':
		case 'hour':
			return num * 60 * 60 * 1000;
		case 'days':
		case 'day':
			return num * 24 * 60 * 60 * 1000;
		default:
			return 60 * 60 * 1000; // default to 1 hour
	}
}
export const load = async ({ params, url, depends }) => {
	depends('app:systems');
	// Extract the path from the params
	const path = params.path;
	const path_number = Number(path);
	const timeRange = url.searchParams.get('range') || '1 hour';
	// Get interval in minutes from query params (default to 5)
	const intervalMinutes = Number(url.searchParams.get('interval')) || 5;
	const intervalSeconds = intervalMinutes * 60;
	// Calculate the start time based on the selected range
	const now = new Date();

	// Calculate the start time by subtracting the interval
	const startTime = new Date(now.getTime() - parseTimeRangeToMs(timeRange));
	// find system using the path
	let system = await db.query.systems.findFirst({
		where: (systems, { eq }) => eq(systems.id, path_number),
		with: {
			metrics: {
				orderBy: (metrics, {desc}) => desc(metrics.time),
				limit: 15
			}, // Include metrics if needed
			disks: {
				where: (disks, {gte}) => gte(disks.time, startTime.toISOString()),
			}, // Include disks if needed
		}
	})

	if (!system) {
		error(404, 'System not found');
	}
	console.log(`Fetching metrics for system ${system.label} (${system.id}) from ${startTime.toISOString()} to ${now.toISOString()} with interval of ${intervalMinutes} minutes`);

	const metrics = await db.execute(sql`
SELECT
	to_timestamp(
		floor(
			extract(epoch from time) / ${intervalSeconds}
		) * ${intervalSeconds}
	) as time_slot,
	AVG(cpu_usage) as cpu_total,
	AVG(memory_used_kb) as used_total,
	AVG(net_in) as in_total,
	AVG(net_out) as out_total,
	AVG(load_one) as one_total,
	AVG(load_five) as five_total,
	AVG(load_fifteen) as fifteen_total,
	AVG(memory_total_kb) as memory_total_kb
FROM metrics
WHERE
	system_id = ${path_number}
	AND time >= ${startTime.toISOString()}
GROUP BY time_slot
ORDER BY time_slot ASC
`);

	const disks = await db.execute(sql`
SELECT
	mount_point,
	to_timestamp(
		floor(
			extract(epoch from time) / ${intervalSeconds}
		) * ${intervalSeconds}
	) as time_slot,
	AVG(used) as used_total,
	AVG(space) as total,
	AVG(read) as read_total,
	AVG(write) as write_total
FROM disks
WHERE
	system = ${path_number}
	AND time >= ${startTime.toISOString()}
GROUP BY mount_point, time_slot
ORDER BY time_slot ASC
`)

	console.log("REFRESHED SYSTEM PAGE");

	// Return the path as a prop
	return {
		system: system,
		metrics: metrics,
		disks: disks
	};
};
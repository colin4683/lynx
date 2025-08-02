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
function timeStringToTodayDate(time: string): Date {
	const [hours, minutes] = time.split(':').map(Number);
	const now = new Date();
	now.setHours(hours, minutes, 0, 0);
	return now;
}

export const load = async ({ params, url, depends }) => {
	depends('app:systems');
	// Extract the path from the params
	const path = params.path;
	const path_number = Number(path);
	// Get interval in minutes from query params (default to 5)
	const intervalMinutes = Number(url.searchParams.get('interval')) || 5;
	const intervalSeconds = intervalMinutes * 60;
	// Calculate the start time based on the selected range

	// Parse date range
	let startTime: Date;
	let endTime: Date;

	const startDateParam = url.searchParams.get("startDate");
	const endDateParam = url.searchParams.get("endDate");
	let timeRange = url.searchParams.get('range') || '1 hour';

	if (startDateParam && endDateParam) {
		startTime = timeStringToTodayDate(startDateParam);
		endTime = timeStringToTodayDate(endDateParam);
		if (startTime > endTime) {
			error(400, 'Start date cannot be after end date');
		}
		if (endTime > new Date()) {
			error(400, 'End date cannot be in the future');
		}
	} else {
		endTime = new Date();
		if (timeRange === 'Live') {
			timeRange = '15 minutes';
		}
		startTime = new Date(endTime.getTime() - parseTimeRangeToMs(timeRange));
	}

	// Calculate the start time by subtracting the interval
	// find system using the path
	let system = await db.query.systems.findFirst({
		where: (systems, { eq }) => eq(systems.id, path_number),
		with: {
			metrics: {
				orderBy: (metrics, { desc }) => desc(metrics.time),
				limit: 15
			}, // Include metrics if needed
			disks: {
				where: (disks, { gte }) => gte(disks.time, startTime.toISOString())
			} // Include disks if needed
		}
	});

	if (!system) {
		error(404, 'System not found');
	}
	console.log(
		`Fetching metrics for system ${system.label} (${system.id}) from ${startTime.toISOString()} to ${endTime.toISOString()} with interval of ${intervalMinutes} minutes`
	);

	// Get the user's local timezone from the browser (if passed as a query param), otherwise use server's default
	const tz = url.searchParams.get('tz') || Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
	console.log(tz);
	let metrics;
	if (intervalMinutes === 1) {
		metrics = await db.execute(sql`
			SELECT
				to_char(time, 'YYYY-MM-DD HH24:MI:SS') as time_slot,
				cpu_usage as cpu_total,
				memory_used_kb as used_total,
				net_in as in_total,
				net_out as out_total,
				load_one as one_total,
				load_five as five_total,
				load_fifteen as fifteen_total,
				memory_total_kb,
				components
			FROM metrics
			WHERE
				system_id = ${path_number}
				AND time >= ${startTime.toISOString()}
				AND time <= ${endTime.toISOString()}
			ORDER BY time ASC
		`);
	} else {
		metrics = await db.execute(sql`
			WITH time_slots AS (
				SELECT
					to_char(
						to_timestamp(
							floor(
								extract(epoch from time) / ${intervalSeconds}
							) * ${intervalSeconds}
						),
						'YYYY-MM-DD HH24:MI:SS'
					) as time_slot,
					*
				FROM metrics
				WHERE
					system_id = ${path_number}
					AND time >= ${startTime.toISOString()}
					AND time <= ${endTime.toISOString()}
			),
			component_stats AS (
		SELECT DISTINCT ON (time_slot, component->>'label')
			time_slot,
			component->>'label' as component_name,
			AVG((component->>'temperature')::numeric) as avg_temp
		FROM time_slots,
			jsonb_array_elements(components::jsonb) as component
		GROUP BY time_slot, component_name
		)
		SELECT
			t.time_slot,
			AVG(cpu_usage) as cpu_total,
			AVG(memory_used_kb) as used_total,
			AVG(net_in) as in_total,
			AVG(net_out) as out_total,
			AVG(load_one) as one_total,
			AVG(load_five) as five_total,
			AVG(load_fifteen) as fifteen_total,
			AVG(memory_total_kb) as memory_total_kb,
			(
				SELECT jsonb_agg(jsonb_build_object(
					'label', cs.component_name,
					'avg_temperature', cs.avg_temp
							 ))
				FROM component_stats cs
				WHERE cs.time_slot = t.time_slot
			) as component_temperatures
		FROM time_slots t
		GROUP BY t.time_slot
		ORDER BY t.time_slot ASC
		`);
	}

	let disks;
	if (intervalMinutes === 1) {
		disks = await db.execute(sql`
			SELECT
				mount_point,
				to_char(time, 'YYYY-MM-DD HH24:MI:SS') as time_slot,
				used as used_total,
				space as total,
				read as read_total,
				write as write_total
			FROM disks
			WHERE
				system = ${path_number}
				AND mount_point = '/'
				AND time >= ${startTime.toISOString()}
				AND time <= ${endTime.toISOString()}
			ORDER BY time ASC
		`);
	} else {
		disks = await db.execute(sql`
			SELECT
				mount_point,
				to_char(
					to_timestamp(
						floor(
							extract(epoch from time) / ${intervalSeconds}
						) * ${intervalSeconds}
					),
					'YYYY-MM-DD HH24:MI:SS'
				) as time_slot,
				AVG(used) as used_total,
				AVG(space) as total,
				AVG(read) as read_total,
				AVG(write) as write_total
			FROM disks
			WHERE
				system = ${path_number}
				AND mount_point = '/'
				AND time >= ${startTime.toISOString()}
				AND time <= ${endTime.toISOString()}
			GROUP BY mount_point, time_slot
			ORDER BY time_slot ASC
		`);
	}
	// Return the path as a prop
	return {
		system: system,
		metrics: metrics,
		disks: disks
	};
};

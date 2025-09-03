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
	// Extract the path from the params
	const path = params.path;
	const path_number = Number(path);

	const system = await db.query.systems.findFirst({
		where: (systems, { eq }) => eq(systems.id, path_number),
		with: {
			services: true
		}
	});

	if (!system) {
		throw error(404, 'System not found');
	}

	return {
		system,
	};
};

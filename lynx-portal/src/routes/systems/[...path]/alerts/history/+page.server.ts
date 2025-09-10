import { db } from '$lib/server/db';

export const load = async ({ params }) => {
	const path = params.path;
	const path_number = Number(path);

	// Get the system information
	const system = await db.query.systems.findFirst({
		where: (systems, { eq }) => eq(systems.id, path_number)
	});

	// Get complete alert history for this system (last 50 entries)
	const history = await db.query.alertHistory.findMany({
		where: (alertHistory, { eq }) => eq(alertHistory.system, path_number),
		with: {
			alertRule: true
		},
		orderBy: (alertHistory, { desc }) => desc(alertHistory.date),
		limit: 50
	});

	return {
		system,
		path: path_number,
		history
	};
}

import { db } from '$lib/server/db';

export const load = async ({ params, url, depends }) => {
	depends(`app:system:alerts`);
	const path = params.path;
	const path_number = Number(path);

	// Get the system information
	const system = await db.query.systems.findFirst({
		where: (systems, { eq }) => eq(systems.id, path_number)
	});

	// Get all available alert rules
	const availableAlerts = await db.query.alertRules.findMany({
		orderBy: (alertRules, { desc }) => desc(alertRules.updated)
	});

	// Get alert rules currently applied to this system
	const appliedAlerts = await db.query.alertSystems.findMany({
		where: (alertSystems, { eq }) => eq(alertSystems.systemId, path_number),
		with: {
			alertRule: true
		}
	});

	// Get recent alert history for this system (last 10 entries)
	const recentHistory = await db.query.alertHistory.findMany({
		where: (alertHistory, { eq }) => eq(alertHistory.system, path_number),
		with: {
			alertRule: true
		},
		orderBy: (alertHistory, { desc }) => desc(alertHistory.date),
		limit: 10
	});

	return {
		system,
		path: path_number,
		availableAlerts,
		appliedAlerts: appliedAlerts.map(item => item.alertRule),
		recentHistory
	};
}
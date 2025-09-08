import { db } from '$lib/server/db';

export const load = async ({ params, url, depends }) => {
	const path = params.path;
	const path_number = Number(path);

	let alert_history = await db.query.alertHistory.findMany({
		where: (alertHistory, {eq}) => eq(alertHistory.system, path_number),
		with: {
			system: true,
			alertRule: true
		}
	})

	return {
		history: alert_history,
		path: path_number
	}
}
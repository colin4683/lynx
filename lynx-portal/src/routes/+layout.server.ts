import {db} from '../lib/server/db';

export const load = async () => {
	let data = await db.query.systems.findMany();
	let metrics = await db.query.metrics.findMany({
		limit: 10,
		orderBy: (metrics, { desc }) => desc(metrics.time)
	})

	let hub = await db.query.systems.findFirst({
		where: (systems, {eq}) => eq(systems.label, 'lynx-hub')
	});
	if (hub) {
		data = data.filter(system => system.id !== hub.id);
	}

	return {
		systems: data,
		metrics: metrics,
		hub: hub || null
	};
};
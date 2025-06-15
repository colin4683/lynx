import {db} from '../lib/server/db';

export const load = async () => {
	let data = await db.query.systems.findMany({
		with: {
			disks: {
				orderBy: (disks, {desc}) => desc(disks.time)
			},
			metrics: {
				orderBy: (metrics, {desc}) => desc(metrics.time),
				limit: 1
			}
		}
	});
	let metrics = await db.query.metrics.findMany({
		limit: 10,
		orderBy: (metrics, { desc }) => desc(metrics.time),
		with: {
			system: true
		}
	})

	let hub = await db.query.systems.findFirst({
		where: (systems, {eq}) => eq(systems.label, 'lynx-hub'),
		with: {
			disks: {
				orderBy: (disks, {desc}) => desc(disks.time),
				limit: 1,
				where: (disks, {eq}) => eq(disks.mountPoint, "/")
			}
		}
	});
	if (hub) {
		//data = data.filter(system => system.id !== hub.id);
	}

	console.log("REFRESHED LAYOUT PAGE");

	return {
		systems: data,
		metrics: metrics,
		hub: hub || null
	};
};
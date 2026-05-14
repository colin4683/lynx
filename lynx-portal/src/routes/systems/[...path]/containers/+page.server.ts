import { db } from '$lib/server/db';
import { error } from '@sveltejs/kit';
import type { Actions, RequestEvent } from "../$types";

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

	const containers = await db.query.containers.findMany({
		where: (containers, {eq}) => eq(containers.systemId, system.id),
		with: {
			containerMetrics: {
				orderBy: (containerMetrics, { desc }) => [desc(containerMetrics.time)],
			}
		}
	});

	return {
		system,
		containers
	};
};

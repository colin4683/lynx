import { error, json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/server/db';
import { containers } from '$lib/server/db/schema';
import { eq } from 'drizzle-orm';

export const POST: RequestHandler = async ({ params }) => {
	const { id, action } = params;

	if (!['start', 'stop', 'restart'].includes(action)) {
		throw error(400, 'Invalid action');
	}

	try {
		const containerState = action === 'stop' ? 'down' : 'up';
		
		await db.query.containers.findFirst({
			where: (containers, { eq }) => eq(containers.dockerId, id)
		}).then(async (container) => {
			if (!container) {
				throw error(404, 'Container not found');
			}

			await db.update(containers)
				.set({ state: containerState })
				.where(eq(containers.id, container.id));

		});

		return json({ success: true, action, id });
	} catch (e) {
		console.error(`Error performing container action: ${e}`);
		throw error(500, 'Failed to perform container action');
	}
};

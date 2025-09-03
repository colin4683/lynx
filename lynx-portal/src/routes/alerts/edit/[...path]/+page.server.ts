import { db } from '$lib/server/db';
import { alertRules } from '$lib/server/db/schema';
import { error } from '@sveltejs/kit';

export const load = async ({ params, url, depends }) => {

	const path = params.path;
	const path_number = Number(path);
	const alertRule = await db.query.alertRules.findFirst({
		where: (alertRules, {eq}) => eq(alertRules.id, path_number),
		with: {
			alertSystems: {
				with: {
					system: true
				}
			}
		}
	});

	if (!alertRule) {
		error(404, 'Alert rule not found');
	}
	return {
		rule: alertRule
	}
}
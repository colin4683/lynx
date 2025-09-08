import { db } from '$lib/server/db';
import { alertRules } from '$lib/server/db/schema';
import { error } from '@sveltejs/kit';

export const load = async ({ params, url, depends, locals }) => {

	const path = params.path;
	const path_number = Number(path);
	const alertRule = await db.query.alertRules.findFirst({
		where: (alertRules, {eq}) => eq(alertRules.id, path_number),
		with: {
			alertSystems: {
				with: {
					system: true
				}
			},
			alertNotifiers: {
				with: {
					notifier: true
				}
			}
		}
	});

	const notifiers = await db.query.notifiers.findMany({
		where: (notifiers, { eq }) => eq(notifiers.user, locals.user!.id),
		orderBy: (notifiers, { desc }) => desc(notifiers.id)
	})

	if (!alertRule) {
		error(404, 'Alert rule not found');
	}
	return {
		rule: alertRule,
		notifiers: notifiers ?? [],
	}
}
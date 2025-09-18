import type {PageServerLoadEvent} from "./$types";
import { db } from '$lib/server/db';

export const load = async (event: PageServerLoadEvent) => {
	event.depends('app:alerts');

	if (event.locals.session == null || event.locals.user == null) {
		return { redirect: "/login" };
	}

	const alerts = await db.query.alertRules.findMany({
		where: (alerts, { eq }) => eq(alerts.userId, event.locals.user!.id),
		orderBy: (alerts, { desc }) => desc(alerts.created),
		with: {
			alertSystems: {
				with: {
					system: true
				}
			}
		}
	})

	const systems = await db.query.systems.findMany({
		where: (systems, { eq }) => eq(systems.admin, event.locals.user!.id),
		orderBy: (systems, { desc }) => desc(systems.lastSeen)
	})

	const notifiers = await db.query.notifiers.findMany({
		where: (notifiers, { eq }) => eq(notifiers.user, event.locals.user!.id),
		orderBy: (notifiers, { desc }) => desc(notifiers.id)
	})

	return {
		alerts: alerts,
		systems: systems,
		notifiers: notifiers,
		user: event.locals.user
	};
}

import type {Actions, PageServerLoadEvent, RequestEvent} from "./$types";
import { db } from '$lib/server/db';

export const load = async (event: PageServerLoadEvent) => {
	event.depends('app:alerts');

	if (event.locals.session == null || event.locals.user == null) {
		return { redirect: "/login" };
	}

	if (!event.locals.user.emailVerified) {
		return { redirect: "/verify-email" };
	}

	if (!event.locals.user.registered2FA) {
		return { redirect: "/2fa/setup" };
	}

	if (!event.locals.session.twoFactorVerified) {
		return { redirect: "/2fa" };
	}

	const alerts = await db.query.alertRules.findMany({
		where: (alerts, { eq }) => eq(alerts.userId, event.locals.user!.id),
		orderBy: (alerts, { desc }) => desc(alerts.created)
	})

	return {
		alerts: alerts,
		user: event.locals.user
	};
}
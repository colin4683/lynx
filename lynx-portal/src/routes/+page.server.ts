import type {Actions, PageServerLoadEvent, RequestEvent} from "./$types";
import { redirect } from '@sveltejs/kit';
import { db } from '$lib/server/db';

export async function load(event: PageServerLoadEvent) {


	let users = await db.query.users.findFirst({
		where: (users, {eq}) => eq(users.admin, true)
	});

	if (!users) {
		return redirect(302, "/register");
	}

	if (event.locals.session == null || event.locals.user == null) {
		return redirect(302, "/login");
	}
	if (!event.locals.user.emailVerified) {
		return redirect(302, "/verify-email");
	}
	if (!event.locals.user.registered2FA) {
		return redirect(302, "/2fa/setup");
	}
	if (!event.locals.session.twoFactorVerified) {
		return redirect(302, "/2fa");
	}

	// get alert_history for systems where system.admin is current user id
	// alert_history has (
	const systemIds = await db.query.systems.findMany({
		where: (systems, { eq }) => eq(systems.admin, event.locals.user!.id),
		columns: { id: true }
	});

	const ids = systemIds.map(s => s.id);
	const alertHistory = await db.query.alertHistory.findMany({
		where: (alertHistory, { inArray }) => inArray(alertHistory.system, ids),
		orderBy: (alertHistory, { desc }) => desc(alertHistory.date),
		with: {
			system: true,
			alertRule: true
		},
	});

	return {
		user: event.locals.user,
		alerts: alertHistory
	};
}
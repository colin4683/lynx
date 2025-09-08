import type { Actions } from './$types';
import { redirect, type RequestEvent } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { alertRules } from '$lib/server/db/schema';
import type { PageServerLoadEvent } from '../../../../.svelte-kit/types/src/routes/$types';




export const actions: Actions = {
	default: action
};

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

	const notifiers = await db.query.notifiers.findMany({
		where: (notifiers, { eq }) => eq(notifiers.user, event.locals.user!.id),
		orderBy: (notifiers, { desc }) => desc(notifiers.id)
	})

	return {
		notifiers: notifiers ?? [],
		user: event.locals.user
	};
}

async function action(event: RequestEvent) {
	// save new alert rule
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

	const formData = await event.request.formData();
	const name = formData.get("name") as string | null;
	const description = formData.get("description") as string | null;
	const expression = formData.get("expression") as string | null;
	const severity = formData.get("severity") as string | null;

	if (!name || !expression || !severity) {
		return { success: false, message: "Name, expression and severity are required" };
	}


	const alert = await db.query.alertRules.findFirst({
		where: (alerts, { eq }) => eq(alerts.name, name)
	});
	if (alert) {
		return { success: false, message: "Alert rule with this name already exists" };
	}



	const newAlert = await db.insert(alertRules).values({
		name: name,
		description: description,
		expression,
		severity,
		userId: event.locals.user.id
	}).returning();

	if (newAlert.length === 0) {
		return { success: false, message: "Failed to create alert rule" };
	}
	return { success: true, message: "Alert rule created successfully", alert: newAlert[0] };
}
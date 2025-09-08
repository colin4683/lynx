import type {Actions, PageServerLoadEvent, RequestEvent} from "../$types";
import { db } from '$lib/server/db';
import { notifiers } from '$lib/server/db/schema';
import { eq } from 'drizzle-orm';
import { json } from '@sveltejs/kit';

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
import type { Actions, PageServerLoadEvent, RequestEvent } from "./$types";
import { fail, redirect } from '@sveltejs/kit';
import { db } from '$lib/server/db';

export async function load(event: PageServerLoadEvent) {

	let users = await db.query.users.findFirst({
		where: (users, {eq}) => eq(users.admin, true)
	});

	if (users) {
		return redirect(302, "/login");
	}

	if (event.locals.session != null && event.locals.user != null) {
		if (!event.locals.user.emailVerified) {
			return redirect(302, "/verify-email");
		}
		if (!event.locals.user.registered2FA) {
			return redirect(302, "/2fa/setup");
		}
		if (!event.locals.session.twoFactorVerified) {
			return redirect(302, "/2fa");
		}
		return redirect(302, "/");
	}
	return {};
}

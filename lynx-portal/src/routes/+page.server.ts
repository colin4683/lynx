import type {Actions, PageServerLoadEvent, RequestEvent} from "./$types";
import { redirect } from '@sveltejs/kit';

export function load(event: PageServerLoadEvent) {
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
	return {
		user: event.locals.user
	};
}
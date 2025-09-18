import type { Actions, PageServerLoadEvent, RequestEvent } from "./$types";
import { fail, redirect } from '@sveltejs/kit';
import { hashPassword, login } from '$lib/server/auth';
import { createSession, generateSessionToken, type SessionFlags, setSessionCookie } from '$lib/server/session';

export function load(event: PageServerLoadEvent) {
	if (event.locals.session != null && event.locals.user != null) {
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

export const actions: Actions = {
	default: action
};

async function action(event: RequestEvent) {
	const clientIp = event.request.headers.get("X-Forwarded-For");

	const formData = await event.request.formData();
	const email = formData.get("email");
	const password = formData.get("password");
	if (typeof email !== "string" || typeof password !== "string") {
		return fail(400, {
			message: "Invalid form data",
			email: ""
		})
	}
	if (email.length === 0 || password.length === 0) {
		return fail(400, {
			message: "Email and password are required",
			email: email
		});
	}

	if (!(/^.+@.+\..+$/.test(email)) || email.length > 255) {
		return fail(400, {
			message: "Invalid email address",
			email: email
		});
	}

	let test = await hashPassword(password);

	let authorized = await login(email, password);
	if (!authorized.success || authorized.error || !authorized.userId) {
		return fail(400, {
			message: authorized.error ?? "Unknown error occurred",
			email: email
		})
	}

	const sessionFlags: SessionFlags = {
		twoFactorVerified: 0
	};
	const sessionToken = generateSessionToken();
	const session = await createSession(sessionToken, authorized.userId, sessionFlags);
	setSessionCookie(event, sessionToken, new Date(session.expiresAt * 1000));


	return redirect(301, "/");
}
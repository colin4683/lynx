import type { Actions, PageServerLoadEvent, RequestEvent } from "./$types";
import { fail, redirect } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { hashPassword, login, register } from '$lib/server/auth';
import { createSession, generateSessionToken, type SessionFlags, setSessionCookie } from '$lib/server/session';

export async function load(event: PageServerLoadEvent) {

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
	const confirm_password = formData.get("confirm-password");
	if (typeof email !== "string" || typeof password !== "string" || typeof confirm_password !== "string") {
		return fail(400, {
			message: "Invalid form data",
			email: ""
		})
	}
	if (email.length === 0 || password.length === 0 || confirm_password.length === 0) {
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

	if (password != confirm_password) {
		return fail(400, {
			message: "Passwords dont match",
			email: email
		})
	}

	let authorized = await register(email, password);
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
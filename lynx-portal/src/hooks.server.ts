import type {Handle} from "@sveltejs/kit";
import { deleteSessionCookie, setSessionCookie, validateToken } from '$lib/server/session';
import { sequence } from '@sveltejs/kit/hooks';

const authHandle: Handle = async ({event, resolve}) => {
	const token = event.cookies.get("session") ?? null;
	if (token === null) {
		event.locals.user = null;
		event.locals.session = null;
		return resolve(event);
	}

	const {session, user} = await validateToken(token);
	if (session !== null) {
		setSessionCookie(event, token, new Date(session.expiresAt * 1000));
	} else {
		deleteSessionCookie(event);
	}

	event.locals.session = session;
	event.locals.user = user;
	return resolve(event);
}

export const handle = sequence(authHandle);
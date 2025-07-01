import { encodeBase32LowerCaseNoPadding, encodeHexLowerCase } from '@oslojs/encoding';
import { sha256 } from '@oslojs/crypto/sha2';
import type { RequestEvent } from '@sveltejs/kit';
import type { User } from './user';
import { db } from '$lib/server/db';
import { sessions } from '$lib/server/db/schema';
import { eq } from 'drizzle-orm';

type SessionValidationResult = { session: Session; user: User } | { session: null; user: null };
export interface SessionFlags {
	twoFactorVerified: number;
}

export interface Session extends SessionFlags {
	id: string;
	expiresAt: number;
	userId: number;
}

export async function validateToken(token: string): Promise<SessionValidationResult> {
	const sessionId = encodeHexLowerCase(sha256(new TextEncoder().encode(token)));
	const session = await db.query.sessions.findFirst({
		where: (sessions, { eq }) => eq(sessions.id, sessionId),
		with: {
			user: {
				columns: {
					password: false
				}
			}
		}
	});

	if (!session) {
		return { session: null, user: null };
	}

	// remove session if expired
	if (Date.now() >= new Date(session.expiresAt * 1000).getTime()) {
		await db.delete(sessions).where(eq(sessions.id, sessionId));
		return { session: null, user: null };
	}

	// update session if about to expire
	if (Date.now() >= new Date(session.expiresAt * 1000).getTime() - 1000 * 60 * 60 * 24 * 15) {
		let newExpire = Math.floor(new Date(Date.now() + 1000 * 60 * 60 * 24 * 30).getTime() / 1000);
		await db.update(sessions).set({ expiresAt: newExpire });
	}

	return { session, user: {
		...session.user,
			registered2FA: session.user.totpKey != null
		} };
}

export async function removeSession(sessionId: string): Promise<void> {
	await db.delete(sessions).where(eq(sessions.id, sessionId));
}

export function setSessionCookie(event: RequestEvent, token: string, expiresAt: Date): void {
	event.cookies.set('session', token, {
		httpOnly: true,
		path: '/',
		secure: import.meta.env.PDO,
		sameSite: 'lax',
		expires: expiresAt
	});
}

export function deleteSessionCookie(event: RequestEvent): void {
	event.cookies.set('session', '', {
		httpOnly: true,
		path: '/',
		secure: import.meta.env.PDO,
		sameSite: 'lax',
		maxAge: 0
	});
}

export function generateSessionToken(): string {
	const tokenBytes = new Uint8Array(20);
	crypto.getRandomValues(tokenBytes);
	return encodeBase32LowerCaseNoPadding(tokenBytes).toLowerCase();
}


export async function createSession(token: string, userId: number, flags: SessionFlags): Promise<Session> {
	const sessionId = encodeHexLowerCase(sha256(new TextEncoder().encode(token)));
	let expires = Math.floor(new Date(Date.now() + 1000 * 60 * 60 * 24 * 30).getTime() / 1000);
	const session: Session = {
		id: sessionId,
		userId,
		expiresAt: expires,
		twoFactorVerified: flags.twoFactorVerified
	};

	await db.insert(sessions).values({
		id: sessionId,
		userId,
		expiresAt: expires,
		twoFactorVerified: flags.twoFactorVerified
	});

	return session;
}

export async function setSession2fa(sessionId: string): Promise<void> {
	await db.update(sessions).set({twoFactorVerified: 1}).where(eq(sessions.id, sessionId));
}
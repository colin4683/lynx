import { json, redirect, type RequestEvent, type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { alertRules, notifiers as notifiersdb } from '$lib/server/db/schema';
import { eq } from 'drizzle-orm';

export const POST: RequestHandler = async (event: RequestEvent) => {
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

	// Parse form body from json
	if (event.locals.session == null || event.locals.user == null) {
		return json({ error: 'Unauthorized' }, { status: 401 });
	}
	const { notifiers } = await event.request.json();
	const userId = event.locals.user.id;
	try {
		// Insert new notifiers with URL format
		for (const notification of notifiers) {
			const existingNotifier = await db.query.notifiers.findFirst({
				where: (notifiersdb, { and, eq }) => and(
					eq(notifiersdb.user, userId),
					eq(notifiersdb.type, notification.type)
				)
			});
			if (existingNotifier) {
				// Update existing notifier
				await db.update(notifiersdb).set({
					value: notification.url // Update URL directly instead of JSON
				}).where(eq(notifiersdb.id, existingNotifier.id));
				continue; // Skip to next notifier
			}
			await db.insert(notifiersdb).values({
				user: userId,
				type: notification.type,
				value: notification.url // Store URL directly instead of JSON
			});
		}

		return json({ success: true });
	} catch (error) {
		console.error('Error saving notification settings:', error);
		return json({ error: 'Failed to save notification settings' }, { status: 500 });
	}

}
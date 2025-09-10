import { json, redirect, type RequestEvent, type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { alertRules, alertSystems, notifiers as notifiersdb } from '$lib/server/db/schema';
import { and, eq } from 'drizzle-orm';

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
	const { alerts, system } = await event.request.json();

	const userId = event.locals.user.id;

	if (!system || typeof system !== 'number') {
		return json({ error: 'Invalid system ID' }, { status: 400 });
	}

	try {
		// Insert new alert rules
		for (const alertId of alerts) {
			if (typeof alertId !== 'number') {
				continue; // Skip invalid IDs
			}
			// check if rule exists and belongs to user
			const rule = await db.query.alertRules.findFirst({
				where: (alertRules, { and, eq }) => and(
					eq(alertRules.id, alertId),
					eq(alertRules.userId, userId)
				)
			});
			if (!rule) {
				continue; // Skip if rule doesn't exist or doesn't belong to user
			}

			// Check if the alert is already applied to the system
			const existingAlert = await db.query.alertSystems.findFirst({
				where: (alertSystems, { and, eq }) => and(
					eq(alertSystems.ruleId, alertId),
					eq(alertSystems.systemId, system)
				)
			});
			if (existingAlert) {
				continue; // Skip if already applied
			}

			await db.insert(alertSystems).values({
				ruleId: alertId,
				systemId: system
			});
		}

		return json({ success: true });
	} catch (error) {
		console.error('Error saving alert settings:', error);
		return json({ error: 'Failed to save alert settings' }, { status: 500 });
	}
}

export const DELETE : RequestHandler = async (event: RequestEvent) => {
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
	const { alertId, system } = await event.request.json();

	const userId = event.locals.user.id;

	if (!system || typeof system !== 'number') {
		return json({ error: 'Invalid system ID' }, { status: 400 });
	}

	if (!alertId || typeof alertId !== 'number') {
		return json({ error: 'Invalid alert ID' }, { status: 400 });
	}

	try {
		// check if rule exists and belongs to user
		const rule = await db.query.alertRules.findFirst({
			where: (alertRules, { and, eq }) => and(
				eq(alertRules.id, alertId),
				eq(alertRules.userId, userId)
			)
		});
		if (!rule) {
			return json({ error: 'Alert rule not found' }, { status: 404 });
		}

		// Delete the alert-system association
		await db.delete(alertSystems).where(and(
			eq(alertSystems.ruleId, alertId),
			eq(alertSystems.systemId, system)
		));

		return json({ success: true });
	} catch (error) {
		console.error('Error deleting alert setting:', error);
		return json({ error: 'Failed to delete alert setting' }, { status: 500 });
	}
}
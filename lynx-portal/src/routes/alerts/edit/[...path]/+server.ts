import { redirect, type RequestEvent, type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { alertNotifiers, alertRules } from '$lib/server/db/schema';
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
	const body = await event.request.json();
	const name = body.name as string | null;
	const description = body.description as string | null;
	const expression = body.expression as string | null;
	const severity = body.severity as string | null;
	const notifiers = body.notifiers as number[] ?? [];
	if (!name || !expression || !severity) {
		return new Response("Name, expression and severity are required", {
			status: 400,
			headers: {
				"Content-Type": "text/plain"
			}
		});
	}

	const alert = await db.query.alertRules.findFirst({
		where: (alerts, { eq }) => eq(alerts.name, name)
	});
	if (!alert) {
		return new Response("Alert not found.", {
			status: 400,
			headers: {
				"Content-Type": "text/plain"
			}
		});
	}

	await db.update(alertRules).set({
		name: name,
		description: description,
		expression,
		severity,
		updated: new Date().toISOString()
	}).where(eq(alertRules.id, alert.id));


	for (const notifierId of notifiers) {
		const existing = await db.query.alertNotifiers.findFirst({
			where: (an, { and, eq }) => and(
				eq(an.ruleId, alert.id),
				eq(an.notifierId, notifierId)
			)
		});
		if (!existing) {
			await db.insert(alertNotifiers).values({
				ruleId: alert.id,
				notifierId: notifierId
			});
		}
	}


	return new Response("Alert rule saved", {
		status: 201,
		headers: {
			"Content-Type": "text/plain"
		}
	});
}
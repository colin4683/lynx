import { redirect, type RequestEvent, type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { alertRules } from '$lib/server/db/schema';

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
	if (!name || !expression || !severity) {
		console.log("Missing required fields:", { name, expression, severity });
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
	if (alert) {
		console.log("Alert rule with this name already exists:", name);
		return new Response("Alert rule with this name already exists", {
			status: 400,
			headers: {
				"Content-Type": "text/plain"
			}
		});
	}



	const newAlert = await db.insert(alertRules).values({
		name: name,
		description: description,
		expression,
		severity,
		userId: event.locals.user.id
	}).returning();

	if (newAlert.length === 0) {
		return new Response("Failed to create alert rule", {
			status: 500,
			headers: {
				"Content-Type": "text/plain"
			}
		});
	}

	return new Response("Alert rule created successfully", {
		status: 201,
		headers: {
			"Content-Type": "text/plain"
		}
	});
}
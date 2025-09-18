import { redirect, type RequestEvent, type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/server/db';
import { alertRules } from '$lib/server/db/schema';
import { and, eq } from 'drizzle-orm';

export const POST: RequestHandler = async (event: RequestEvent) => {
	if (event.locals.session == null || event.locals.user == null) {
		return redirect(302, "/login");
	}


	// Parse form body from json
	const body = await event.request.json();
	const {active, id} = body;

	// update alert rule in db
	let updatedRule = await db.update(alertRules).set({
		active: active
	}).where(and(
		eq(alertRules.id, id),
		eq(alertRules.userId, event.locals.user.id)
	))
		.returning();

	if (updatedRule.length === 0) {
		return new Response("Alert rule not found", {
			status: 404,
			headers: {
				"Content-Type": "text/plain"
			}
		});
	}


	return new Response("Alert rule enabled", {
		status: 201,
		headers: {
			"Content-Type": "text/plain"
		}
	});
}

export const DELETE: RequestHandler = async (event: RequestEvent) => {
	if (event.locals.session == null || event.locals.user == null) {
		return redirect(302, "/login");
	}
	const body = await event.request.json();
	const {id} = body;

	// delete alert rule from db
	let deletedRule = await db.delete(alertRules).where(and(
		eq(alertRules.id, id),
		eq(alertRules.userId, event.locals.user.id)
	))
		.returning();

	if (deletedRule.length === 0) {
		return new Response("Alert rule not found", {
			status: 404,
			headers: {
				"Content-Type": "text/plain"
			}
		});
	}

	return new Response("Alert rule deleted", {
		status: 200,
		headers: {
			"Content-Type": "text/plain"
		}
	});
}
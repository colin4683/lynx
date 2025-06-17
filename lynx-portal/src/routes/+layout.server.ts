import {db} from '../lib/server/db';
import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoadEvent } from './$types';

export async function load(event: LayoutServerLoadEvent) {

	let users = await db.query.users.findFirst({
		where: (users, {eq}) => eq(users.admin, true)
	});

	if (!users) {
		return redirect(302, "/register");
	}

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

	let data = await db.query.systems.findMany({
		with: {
			disks: {
				orderBy: (disks, {desc}) => desc(disks.time)
			},
			metrics: {
				orderBy: (metrics, {desc}) => desc(metrics.time),
				limit: 1
			}
		}
	});
	let metrics = await db.query.metrics.findMany({
		limit: 10,
		orderBy: (metrics, { desc }) => desc(metrics.time),
		with: {
			system: true
		}
	})

	let hub = await db.query.systems.findFirst({
		where: (systems, {eq}) => eq(systems.label, 'lynx-hub'),
		with: {
			disks: {
				orderBy: (disks, {desc}) => desc(disks.time),
				limit: 1,
				where: (disks, {eq}) => eq(disks.mountPoint, "/")
			}
		}
	});
	if (hub) {
		//data = data.filter(system => system.id !== hub.id);
	}

	return {
		systems: data,
		metrics: metrics,
		hub: hub || null,
		user: event.locals.user
	};
};
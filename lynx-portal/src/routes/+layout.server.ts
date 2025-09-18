import {db} from '../lib/server/db';
import {redirect} from '@sveltejs/kit';
import type {LayoutServerLoadEvent} from './$types';

export async function load(event: LayoutServerLoadEvent) {
    let user = await db.query.users.findFirst({
        where: (users, {eq}) => eq(users.admin, true)
    });
    let login_route = event.request.url.includes('/login');
    let setup_route = event.request.url.includes('/setup');
    let otp_route = event.request.url.includes('/2fa');
    if (!login_route && !setup_route && !otp_route) {

        // Should have default user seeded during initial setup
        // this should never happen so die here
        if (!user) {
            throw new Error("No admin user found, please re-seed the database");
        }
				if (event.locals.session == null || event.locals.user == null) {
            return redirect(302, "/login");
        }
				if (event.locals.user.email=="admin@system.lynx") {
						return redirect(302, "/setup");
				}
        if (!event.locals.user.registered2FA) {
            return redirect(302, "/2fa/setup");
        }
        if (!event.locals.session.twoFactorVerified) {
            return redirect(302, "/2fa");
        }
    }

    let range = new Date(Date.now() - 1000 * 60 * 30).toISOString(); // 30 minutes ago
    let data = await db.query.systems.findMany({
        where: (systems, {eq}) => eq(systems.admin, event.locals.user?.id || 0),
        with: {
            disks: {
                orderBy: (disks, {desc}) => desc(disks.time)
            },
            metrics: {
                orderBy: (metrics, {desc}) => desc(metrics.time),
                limit: 1
            },
            alertHistories: {
                where: (alertHistories, {gte}) => gte(alertHistories.date, range),
                orderBy: (alertHistories, {desc}) => desc(alertHistories.date),
                limit: 1
            }
        }
    });
    let metrics = await db.query.metrics.findMany({
        where: (metrics, {eq}) => eq(metrics.systemId, data[0]?.id || 0),
        limit: 10,
        orderBy: (metrics, {desc}) => desc(metrics.time),
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
        user: event.locals.user,
    };
}
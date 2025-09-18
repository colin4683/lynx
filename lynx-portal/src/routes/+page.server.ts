import type {PageServerLoadEvent} from "./$types";
import {redirect} from '@sveltejs/kit';
import {db} from '$lib/server/db';

export async function load(event: PageServerLoadEvent) {
    let users = await db.query.users.findFirst({
        where: (users, {eq}) => eq(users.admin, true)
    });


    // Get systems with their latest metrics, disks, and alert history
    const systems = await db.query.systems.findMany({
        where: (systems, {eq}) => eq(systems.admin, event.locals.user!.id),
        with: {
            disks: {
                orderBy: (disks, {desc}) => desc(disks.time),
                limit: 5,
                where: (disks, {eq}) => eq(disks.mountPoint, "/")
            },
            metrics: {
                orderBy: (metrics, {desc}) => desc(metrics.time),
                limit: 1
            },
            alertHistories: {
                where: (alertHistories, {gte}) => {
                    const thirtyMinutesAgo = new Date(Date.now() - 30 * 60 * 1000).toISOString();
                    return gte(alertHistories.date, thirtyMinutesAgo);
                },
                orderBy: (alertHistories, {desc}) => desc(alertHistories.date),
                limit: 5,
                with: {
                    alertRule: true
                }
            }
        }
    });

    // Get recent metrics for trends
    const metrics = await db.query.metrics.findMany({
        where: (metrics, {eq}) => eq(metrics.systemId, systems[0]?.id || 0),
        orderBy: (metrics, {desc}) => desc(metrics.time),
        limit: 24,
        with: {
            system: true
        }
    });

    // Get alert history for systems owned by current user
    const systemIds = systems.map(s => s.id);
    const alertHistory = await db.query.alertHistory.findMany({
        where: (alertHistory, {inArray}) => inArray(alertHistory.system, systemIds),
        orderBy: (alertHistory, {desc}) => desc(alertHistory.date),
        limit: 50,
        with: {
            system: true,
            alertRule: true
        },
    });

    return {
        user: event.locals.user,
        systems,
        metrics,
        alerts: alertHistory
    };
}
import type {PageServerLoadEvent} from "../$types";
import {db} from '$lib/server/db';

export const load = async (event: PageServerLoadEvent) => {
    event.depends('app:alerts');

    if (event.locals.session == null || event.locals.user == null) {
        return {redirect: "/login"};
    }


    const notifiers = await db.query.notifiers.findMany({
        where: (notifiers, {eq}) => eq(notifiers.user, event.locals.user!.id),
        orderBy: (notifiers, {desc}) => desc(notifiers.id)
    })

    return {
        notifiers: notifiers ?? [],
        user: event.locals.user
    };
}
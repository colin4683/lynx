import { relations } from "drizzle-orm/relations";
import { users, sessions, systems, disks, metrics } from "./schema";

export const sessionsRelations = relations(sessions, ({one}) => ({
	user: one(users, {
		fields: [sessions.userId],
		references: [users.id]
	}),
}));

export const usersRelations = relations(users, ({many}) => ({
	sessions: many(sessions),
	systems: many(systems),
}));

export const systemsRelations = relations(systems, ({one, many}) => ({
	user: one(users, {
		fields: [systems.admin],
		references: [users.id]
	}),
	disks: many(disks),
	metrics: many(metrics),
}));

export const disksRelations = relations(disks, ({one}) => ({
	system: one(systems, {
		fields: [disks.system],
		references: [systems.id]
	}),
}));

export const metricsRelations = relations(metrics, ({one}) => ({
	system: one(systems, {
		fields: [metrics.systemId],
		references: [systems.id]
	}),
}));
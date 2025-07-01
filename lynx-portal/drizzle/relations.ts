import { relations } from "drizzle-orm/relations";
import { users, sessions, alertRules, alertSystems, systems, disks, metrics } from "./schema";

export const sessionsRelations = relations(sessions, ({one}) => ({
	user: one(users, {
		fields: [sessions.userId],
		references: [users.id]
	}),
}));

export const usersRelations = relations(users, ({many}) => ({
	sessions: many(sessions),
	alertRules: many(alertRules),
	systems: many(systems),
}));

export const alertRulesRelations = relations(alertRules, ({one, many}) => ({
	user: one(users, {
		fields: [alertRules.userId],
		references: [users.id]
	}),
	alertSystems: many(alertSystems),
}));

export const alertSystemsRelations = relations(alertSystems, ({one}) => ({
	alertRule: one(alertRules, {
		fields: [alertSystems.ruleId],
		references: [alertRules.id]
	}),
	system: one(systems, {
		fields: [alertSystems.systemId],
		references: [systems.id]
	}),
}));

export const systemsRelations = relations(systems, ({one, many}) => ({
	alertSystems: many(alertSystems),
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
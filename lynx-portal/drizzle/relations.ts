import { relations } from "drizzle-orm/relations";
import { users, sessions, alertRules, alertSystems, systems, services, notifiers, disks, metrics, alertHistory } from "./schema";

export const sessionsRelations = relations(sessions, ({one}) => ({
	user: one(users, {
		fields: [sessions.userId],
		references: [users.id]
	}),
}));

export const usersRelations = relations(users, ({many}) => ({
	sessions: many(sessions),
	alertRules: many(alertRules),
	notifiers: many(notifiers),
	systems: many(systems),
}));

export const alertRulesRelations = relations(alertRules, ({one, many}) => ({
	user: one(users, {
		fields: [alertRules.userId],
		references: [users.id]
	}),
	alertSystems: many(alertSystems),
	alertHistories: many(alertHistory),
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
	services: many(services),
	user: one(users, {
		fields: [systems.admin],
		references: [users.id]
	}),
	disks: many(disks),
	metrics: many(metrics),
	alertHistories: many(alertHistory),
}));

export const servicesRelations = relations(services, ({one}) => ({
	system: one(systems, {
		fields: [services.system],
		references: [systems.id]
	}),
}));

export const notifiersRelations = relations(notifiers, ({one}) => ({
	user: one(users, {
		fields: [notifiers.user],
		references: [users.id]
	}),
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

export const alertHistoryRelations = relations(alertHistory, ({one}) => ({
	alertRule: one(alertRules, {
		fields: [alertHistory.alert],
		references: [alertRules.id]
	}),
	system: one(systems, {
		fields: [alertHistory.system],
		references: [systems.id]
	}),
}));
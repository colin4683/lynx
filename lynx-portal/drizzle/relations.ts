import { relations } from "drizzle-orm/relations";
import { users, sessions, alertRules, alertSystems, systems, gpus, gpuMetrics, containers, containerMetrics, services, notifiers, disks, alertNotifiers, metrics, alertHistory } from "./schema";

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
	alertNotifiers: many(alertNotifiers),
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
	gpuses: many(gpus),
	containers: many(containers),
	services: many(services),
	disks: many(disks),
	user: one(users, {
		fields: [systems.admin],
		references: [users.id]
	}),
	metrics: many(metrics),
	alertHistories: many(alertHistory),
}));

export const gpusRelations = relations(gpus, ({one, many}) => ({
	system: one(systems, {
		fields: [gpus.systemId],
		references: [systems.id]
	}),
	gpuMetrics: many(gpuMetrics),
}));

export const gpuMetricsRelations = relations(gpuMetrics, ({one}) => ({
	gpus: one(gpus, {
		fields: [gpuMetrics.gpuId],
		references: [gpus.id]
	}),
}));

export const containerMetricsRelations = relations(containerMetrics, ({one}) => ({
	container: one(containers, {
		fields: [containerMetrics.containerId],
		references: [containers.id]
	}),
}));

export const containersRelations = relations(containers, ({one, many}) => ({
	containerMetrics: many(containerMetrics),
	system: one(systems, {
		fields: [containers.systemId],
		references: [systems.id]
	}),
}));

export const servicesRelations = relations(services, ({one}) => ({
	system: one(systems, {
		fields: [services.system],
		references: [systems.id]
	}),
}));

export const notifiersRelations = relations(notifiers, ({one, many}) => ({
	user: one(users, {
		fields: [notifiers.user],
		references: [users.id]
	}),
	alertNotifiers: many(alertNotifiers),
}));

export const disksRelations = relations(disks, ({one}) => ({
	system: one(systems, {
		fields: [disks.system],
		references: [systems.id]
	}),
}));

export const alertNotifiersRelations = relations(alertNotifiers, ({one}) => ({
	alertRule: one(alertRules, {
		fields: [alertNotifiers.ruleId],
		references: [alertRules.id]
	}),
	notifier: one(notifiers, {
		fields: [alertNotifiers.notifierId],
		references: [notifiers.id]
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
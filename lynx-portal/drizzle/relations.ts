import { relations } from "drizzle-orm/relations";
import { systems, disks, metrics } from "./schema";

export const disksRelations = relations(disks, ({one}) => ({
	system: one(systems, {
		fields: [disks.system],
		references: [systems.id]
	}),
}));

export const systemsRelations = relations(systems, ({many}) => ({
	disks: many(disks),
	metrics: many(metrics),
}));

export const metricsRelations = relations(metrics, ({one}) => ({
	system: one(systems, {
		fields: [metrics.systemId],
		references: [systems.id]
	}),
}));
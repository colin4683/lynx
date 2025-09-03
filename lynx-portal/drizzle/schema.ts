import { pgTable, foreignKey, text, integer, boolean, timestamp, index, serial, unique, doublePrecision, bigint } from "drizzle-orm/pg-core"
import { sql } from "drizzle-orm"



export const sessions = pgTable("sessions", {
	id: text().primaryKey().notNull(),
	userId: integer("user_id").notNull(),
	expiresAt: integer("expires_at").notNull(),
	twoFactorVerified: integer("two_factor_verified").default(0).notNull(),
}, (table) => [
	foreignKey({
			columns: [table.userId],
			foreignColumns: [users.id],
			name: "sessions_users_id_fk"
		}),
]);

export const users = pgTable("users", {
	id: integer().primaryKey().generatedAlwaysAsIdentity({ name: "users_id_seq", startWith: 1, increment: 1, minValue: 1, maxValue: 2147483647, cache: 1 }),
	email: text().notNull(),
	password: text().notNull(),
	admin: boolean().default(false),
	emailVerified: boolean("email_verified").default(false),
	// TODO: failed to parse database type 'bytea'
	totpKey: unknown("totp_key"),
	recoveryCode: text("recovery_code"),
});

export const alertRules = pgTable("alert_rules", {
	id: integer().primaryKey().generatedAlwaysAsIdentity({ name: "alert_rules_id_seq", startWith: 1, increment: 1, minValue: 1, maxValue: 2147483647, cache: 1 }),
	name: text().notNull(),
	description: text(),
	userId: integer("user_id").notNull(),
	expression: text().notNull(),
	severity: text().notNull(),
	active: boolean().default(false),
	created: timestamp({ mode: 'string' }).defaultNow(),
	updated: timestamp({ mode: 'string' }).defaultNow(),
}, (table) => [
	foreignKey({
			columns: [table.userId],
			foreignColumns: [users.id],
			name: "alert_rules_users_id_fk"
		}),
]);

export const alertSystems = pgTable("alert_systems", {
	ruleId: integer("rule_id").notNull(),
	systemId: integer("system_id").notNull(),
}, (table) => [
	index().using("btree", table.systemId.asc().nullsLast().op("int4_ops")),
	foreignKey({
			columns: [table.ruleId],
			foreignColumns: [alertRules.id],
			name: "alert_systems_alert_rules_id_fk"
		}).onUpdate("cascade").onDelete("cascade"),
	foreignKey({
			columns: [table.systemId],
			foreignColumns: [systems.id],
			name: "alert_systems_systems_id_fk"
		}).onUpdate("cascade").onDelete("cascade"),
]);

export const services = pgTable("services", {
	id: serial().notNull(),
	system: integer().notNull(),
	name: text().notNull(),
	description: text(),
	state: text(),
	pid: integer(),
	cpu: text(),
	memory: text(),
}, (table) => [
	foreignKey({
			columns: [table.system],
			foreignColumns: [systems.id],
			name: "services_systems_id_fk"
		}),
]);

export const systems = pgTable("systems", {
	id: serial().primaryKey().notNull(),
	hostname: text(),
	address: text().notNull(),
	lastSeen: timestamp("last_seen", { withTimezone: true, mode: 'string' }),
	key: text(),
	active: boolean().default(false),
	expires: timestamp({ withTimezone: true, mode: 'string' }),
	token: text(),
	label: text().notNull(),
	cpu: text(),
	os: text(),
	kernal: text(),
	cpuCount: integer("cpu_count"),
	cpuUsage: doublePrecision("cpu_usage"),
	uptime: integer(),
	// You can use { mode: "bigint" } if numbers are exceeding js number limitations
	memoryUsed: bigint("memory_used", { mode: "number" }),
	// You can use { mode: "bigint" } if numbers are exceeding js number limitations
	memoryTotal: bigint("memory_total", { mode: "number" }),
	admin: integer(),
}, (table) => [
	foreignKey({
			columns: [table.admin],
			foreignColumns: [users.id],
			name: "systems_users_id_fk"
		}).onUpdate("cascade").onDelete("cascade"),
	unique("systems_hostname_key").on(table.hostname),
]);

export const disks = pgTable("disks", {
	system: integer().notNull(),
	name: text().notNull(),
	space: integer(),
	used: integer(),
	read: doublePrecision(),
	write: doublePrecision(),
	unit: text(),
	time: timestamp({ withTimezone: true, mode: 'string' }).primaryKey().notNull(),
	mountPoint: text("mount_point"),
}, (table) => [
	foreignKey({
			columns: [table.system],
			foreignColumns: [systems.id],
			name: "disks_systems_id_fk"
		}).onUpdate("cascade").onDelete("cascade"),
]);

export const metrics = pgTable("metrics", {
	time: timestamp({ withTimezone: true, mode: 'string' }).notNull(),
	systemId: integer("system_id").notNull(),
	cpuUsage: doublePrecision("cpu_usage"),
	// You can use { mode: "bigint" } if numbers are exceeding js number limitations
	memoryUsedKb: bigint("memory_used_kb", { mode: "number" }),
	// You can use { mode: "bigint" } if numbers are exceeding js number limitations
	memoryTotalKb: bigint("memory_total_kb", { mode: "number" }),
	dockerContainersRunning: integer("docker_containers_running"),
	components: text(),
	uptime: integer(),
	netIn: integer("net_in"),
	netOut: integer("net_out"),
	loadOne: doublePrecision("load_one"),
	loadFive: doublePrecision("load_five"),
	loadFifteen: doublePrecision("load_fifteen"),
}, (table) => [
	index("metrics_time_idx").using("btree", table.time.desc().nullsFirst().op("timestamptz_ops")),
	foreignKey({
			columns: [table.systemId],
			foreignColumns: [systems.id],
			name: "metrics_system_id_fkey"
		}).onUpdate("cascade").onDelete("cascade"),
]);

export const alertHistory = pgTable("alert_history", {
	id: integer().generatedAlwaysAsIdentity({ name: "alert_history_id_seq", startWith: 1, increment: 1, minValue: 1, maxValue: 2147483647, cache: 1 }),
	system: integer().notNull(),
	alert: integer().notNull(),
	date: timestamp({ withTimezone: true, mode: 'string' }).notNull(),
}, (table) => [
	foreignKey({
			columns: [table.alert],
			foreignColumns: [alertRules.id],
			name: "alert_history_alert_rules_id_fk"
		}),
	foreignKey({
			columns: [table.system],
			foreignColumns: [systems.id],
			name: "alert_history_systems_id_fk"
		}),
]);

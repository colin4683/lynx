import { pgTable, foreignKey, integer, text, doublePrecision, timestamp, unique, serial, boolean, bigint, index } from "drizzle-orm/pg-core"
import { sql } from "drizzle-orm"



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
}, (table) => [
	unique("systems_hostname_key").on(table.hostname),
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

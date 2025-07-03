CREATE TABLE "alert_rules" (
	"id" integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY (sequence name "alert_rules_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START WITH 1 CACHE 1),
	"name" text NOT NULL,
	"description" text,
	"user_id" integer NOT NULL,
	"expression" text NOT NULL,
	"severity" text NOT NULL,
	"active" boolean DEFAULT false,
	"created" timestamp DEFAULT now(),
	"updated" timestamp DEFAULT now()
);
--> statement-breakpoint
CREATE TABLE "alert_systems" (
	"rule_id" integer NOT NULL,
	"system_id" integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE "disks" (
	"system" integer NOT NULL,
	"name" text NOT NULL,
	"space" integer,
	"used" integer,
	"read" double precision,
	"write" double precision,
	"unit" text,
	"time" timestamp with time zone PRIMARY KEY NOT NULL,
	"mount_point" text
);
--> statement-breakpoint
CREATE TABLE "metrics" (
	"time" timestamp with time zone NOT NULL,
	"system_id" integer NOT NULL,
	"cpu_usage" double precision,
	"memory_used_kb" bigint,
	"memory_total_kb" bigint,
	"docker_containers_running" integer,
	"components" text,
	"uptime" integer,
	"net_in" integer,
	"net_out" integer,
	"load_one" double precision,
	"load_five" double precision,
	"load_fifteen" double precision
);
--> statement-breakpoint
CREATE TABLE "sessions" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" integer NOT NULL,
	"expires_at" integer NOT NULL,
	"two_factor_verified" integer DEFAULT 0 NOT NULL
);
--> statement-breakpoint
CREATE TABLE "systems" (
	"id" serial PRIMARY KEY NOT NULL,
	"hostname" text,
	"address" text NOT NULL,
	"last_seen" timestamp with time zone,
	"key" text,
	"active" boolean DEFAULT false,
	"expires" timestamp with time zone,
	"token" text,
	"label" text NOT NULL,
	"cpu" text,
	"os" text,
	"kernal" text,
	"cpu_count" integer,
	"cpu_usage" double precision,
	"uptime" integer,
	"memory_used" bigint,
	"memory_total" bigint,
	"admin" integer,
	CONSTRAINT "systems_hostname_key" UNIQUE("hostname")
);
--> statement-breakpoint
CREATE TABLE "users" (
	"id" integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY (sequence name "users_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START WITH 1 CACHE 1),
	"email" text NOT NULL,
	"password" text NOT NULL,
	"admin" boolean DEFAULT false,
	"email_verified" boolean DEFAULT false,
	"totp_key" "bytea",
	"recovery_code" text
);
--> statement-breakpoint
DROP TABLE "user" CASCADE;--> statement-breakpoint
ALTER TABLE "alert_rules" ADD CONSTRAINT "alert_rules_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "alert_systems" ADD CONSTRAINT "alert_systems_alert_rules_id_fk" FOREIGN KEY ("rule_id") REFERENCES "public"."alert_rules"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "alert_systems" ADD CONSTRAINT "alert_systems_systems_id_fk" FOREIGN KEY ("system_id") REFERENCES "public"."systems"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "disks" ADD CONSTRAINT "disks_systems_id_fk" FOREIGN KEY ("system") REFERENCES "public"."systems"("id") ON DELETE cascade ON UPDATE cascade;--> statement-breakpoint
ALTER TABLE "metrics" ADD CONSTRAINT "metrics_system_id_fkey" FOREIGN KEY ("system_id") REFERENCES "public"."systems"("id") ON DELETE cascade ON UPDATE cascade;--> statement-breakpoint
ALTER TABLE "sessions" ADD CONSTRAINT "sessions_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "systems" ADD CONSTRAINT "systems_users_id_fk" FOREIGN KEY ("admin") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE cascade;--> statement-breakpoint
CREATE INDEX "alert_systems_system_id_index" ON "alert_systems" USING btree ("system_id" int4_ops);--> statement-breakpoint
CREATE INDEX "metrics_time_idx" ON "metrics" USING btree ("time" timestamptz_ops);
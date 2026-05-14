CREATE TABLE "alert_history" (
	"id" integer GENERATED ALWAYS AS IDENTITY (sequence name "alert_history_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START WITH 1 CACHE 1),
	"system" integer NOT NULL,
	"alert" integer NOT NULL,
	"date" timestamp with time zone NOT NULL
);
--> statement-breakpoint
CREATE TABLE "alert_notifiers" (
	"rule_id" integer NOT NULL,
	"notifier_id" integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE "notifiers" (
	"id" integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY (sequence name "notifiers_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START WITH 1 CACHE 1),
	"user" integer,
	"type" text NOT NULL,
	"value" text NOT NULL
);
--> statement-breakpoint
CREATE TABLE "services" (
	"id" serial NOT NULL,
	"system" integer NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"state" text,
	"pid" integer,
	"cpu" text,
	"memory" text
);
--> statement-breakpoint
ALTER TABLE "alert_history" ADD CONSTRAINT "alert_history_alert_rules_id_fk" FOREIGN KEY ("alert") REFERENCES "public"."alert_rules"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "alert_history" ADD CONSTRAINT "alert_history_systems_id_fk" FOREIGN KEY ("system") REFERENCES "public"."systems"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "alert_notifiers" ADD CONSTRAINT "alert_notifiers_alert_rules_id_fk" FOREIGN KEY ("rule_id") REFERENCES "public"."alert_rules"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "alert_notifiers" ADD CONSTRAINT "alert_notifiers_notifiers_id_fk" FOREIGN KEY ("notifier_id") REFERENCES "public"."notifiers"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "notifiers" ADD CONSTRAINT "notifiers_users_id_fk" FOREIGN KEY ("user") REFERENCES "public"."users"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "services" ADD CONSTRAINT "services_systems_id_fk" FOREIGN KEY ("system") REFERENCES "public"."systems"("id") ON DELETE no action ON UPDATE no action;
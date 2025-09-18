CREATE TABLE "alert_history"
(
    "id"     integer GENERATED ALWAYS AS IDENTITY (
        sequence name "alert_history_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START
            WITH
            1 CACHE 1
        ),
    "system" integer       NOT NULL,
    "alert"  integer       NOT NULL,
    "date"   timestamp
                 with
                 time zone NOT NULL
);

CREATE TABLE "alert_notifiers"
(
    "rule_id"     integer NOT NULL,
    "notifier_id" integer NOT NULL
);

CREATE TABLE "alert_rules"
(
    "id"          integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY (
        sequence name "alert_rules_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START
            WITH
            1 CACHE 1
        ),
    "name"        text    NOT NULL,
    "description" text,
    "user_id"     integer NOT NULL,
    "expression"  text    NOT NULL,
    "severity"    text    NOT NULL,
    "active"      boolean   DEFAULT false,
    "created"     timestamp DEFAULT now(),
    "updated"     timestamp DEFAULT now()
);

CREATE TABLE "alert_systems"
(
    "rule_id"   integer NOT NULL,
    "system_id" integer NOT NULL
);

CREATE TABLE "disks"
(
    "system"      integer                   NOT NULL,
    "name"        text                      NOT NULL,
    "space"       integer,
    "used"        integer,
    "read"        double precision,
    "write"       double precision,
    "unit"        text,
    "time"        timestamp
                      with
                      time zone PRIMARY KEY NOT NULL,
    "mount_point" text
);

SELECT create_hypertable('disks', 'time', if_not_exists => true);

CREATE TABLE "metrics"
(
    "time"                      timestamp
                                    with
                                    time zone NOT NULL,
    "system_id"                 integer       NOT NULL,
    "cpu_usage"                 double precision,
    "memory_used_kb"            bigint,
    "memory_total_kb"           bigint,
    "docker_containers_running" integer,
    "components"                text,
    "uptime"                    integer,
    "net_in"                    integer,
    "net_out"                   integer,
    "load_one"                  double precision,
    "load_five"                 double precision,
    "load_fifteen"              double precision
);
SELECT create_hypertable('metrics', 'time', if_not_exists => true);

CREATE TABLE "notifiers"
(
    "id"    integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY (
        sequence name "notifiers_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START
            WITH
            1 CACHE 1
        ),
    "user"  integer,
    "type"  text NOT NULL,
    "value" text NOT NULL
);

CREATE TABLE "services"
(
    "id"          serial  NOT NULL,
    "system"      integer NOT NULL,
    "name"        text    NOT NULL,
    "description" text,
    "state"       text,
    "pid"         integer,
    "cpu"         text,
    "memory"      text
);

CREATE TABLE "sessions"
(
    "id"                  text PRIMARY KEY  NOT NULL,
    "user_id"             integer           NOT NULL,
    "expires_at"          integer           NOT NULL,
    "two_factor_verified" integer DEFAULT 0 NOT NULL
);

CREATE TABLE "systems"
(
    "id"           serial PRIMARY KEY NOT NULL,
    "hostname"     text,
    "address"      text               NOT NULL,
    "last_seen"    timestamp
                       with
                       time zone,
    "key"          text,
    "active"       boolean DEFAULT false,
    "expires"      timestamp
                       with
                       time zone,
    "token"        text,
    "label"        text               NOT NULL,
    "cpu"          text,
    "os"           text,
    "kernal"       text,
    "cpu_count"    integer,
    "cpu_usage"    double precision,
    "uptime"       integer,
    "memory_used"  bigint,
    "memory_total" bigint,
    "admin"        integer,
    CONSTRAINT "systems_hostname_key" UNIQUE ("hostname")
);

CREATE TABLE "users"
(
    "id"             integer PRIMARY KEY GENERATED ALWAYS AS IDENTITY (
        sequence name "users_id_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 2147483647 START
            WITH
            1 CACHE 1
        ),
    "email"          text NOT NULL,
    "password"       text NOT NULL,
    "admin"          boolean DEFAULT false,
    "email_verified" boolean DEFAULT false,
    "totp_key"       "bytea",
    "recovery_code"  text
);

ALTER TABLE "alert_history"
    ADD CONSTRAINT "alert_history_alert_rules_id_fk" FOREIGN KEY ("alert") REFERENCES "public"."alert_rules" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "alert_history"
    ADD CONSTRAINT "alert_history_systems_id_fk" FOREIGN KEY ("system") REFERENCES "public"."systems" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "alert_notifiers"
    ADD CONSTRAINT "alert_notifiers_alert_rules_id_fk" FOREIGN KEY ("rule_id") REFERENCES "public"."alert_rules" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "alert_notifiers"
    ADD CONSTRAINT "alert_notifiers_notifiers_id_fk" FOREIGN KEY ("notifier_id") REFERENCES "public"."notifiers" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "alert_rules"
    ADD CONSTRAINT "alert_rules_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "alert_systems"
    ADD CONSTRAINT "alert_systems_alert_rules_id_fk" FOREIGN KEY ("rule_id") REFERENCES "public"."alert_rules" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "alert_systems"
    ADD CONSTRAINT "alert_systems_systems_id_fk" FOREIGN KEY ("system_id") REFERENCES "public"."systems" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "disks"
    ADD CONSTRAINT "disks_systems_id_fk" FOREIGN KEY ("system") REFERENCES "public"."systems" ("id") ON DELETE cascade ON UPDATE cascade;

ALTER TABLE "metrics"
    ADD CONSTRAINT "metrics_system_id_fkey" FOREIGN KEY ("system_id") REFERENCES "public"."systems" ("id") ON DELETE cascade ON UPDATE cascade;

ALTER TABLE "notifiers"
    ADD CONSTRAINT "notifiers_users_id_fk" FOREIGN KEY ("user") REFERENCES "public"."users" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "services"
    ADD CONSTRAINT "services_systems_id_fk" FOREIGN KEY ("system") REFERENCES "public"."systems" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "sessions"
    ADD CONSTRAINT "sessions_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users" ("id") ON DELETE no action ON UPDATE no action;

ALTER TABLE "systems"
    ADD CONSTRAINT "systems_users_id_fk" FOREIGN KEY ("admin") REFERENCES "public"."users" ("id") ON DELETE cascade ON UPDATE cascade;

CREATE INDEX "alert_systems_system_id_index" ON "alert_systems" USING btree ("system_id" int4_ops);

CREATE INDEX IF NOT EXISTS "metrics_time_idx"
    ON "metrics" USING btree ("time" timestamptz_ops);

CREATE FUNCTION public.update_latest_cpu_usage() RETURNS trigger
    LANGUAGE plpgsql
AS
$$
BEGIN
    UPDATE systems
    SET cpu_usage = NEW.cpu_usage
    WHERE id = NEW.system_id;
    RETURN NEW;
END;
$$;
ALTER FUNCTION public.update_latest_cpu_usage() OWNER TO postgres;


CREATE FUNCTION public.update_latest_lastseen() RETURNS trigger
    LANGUAGE plpgsql
AS
$$
BEGIN
    UPDATE systems
    SET last_seen = now()
    WHERE id = NEW.system_id;
    RETURN NEW;
END;
$$;
ALTER FUNCTION public.update_latest_lastseen() OWNER TO postgres;


CREATE FUNCTION public.update_latest_uptime() RETURNS trigger
    LANGUAGE plpgsql
AS
$$
BEGIN
    UPDATE systems
    SET uptime = NEW.uptime
    WHERE id = NEW.system_id;
    RETURN NEW;
END;
$$;
ALTER FUNCTION public.update_latest_uptime() OWNER TO postgres;


CREATE FUNCTION public.update_memory() RETURNS trigger
    LANGUAGE plpgsql
AS
$$
BEGIN
    UPDATE systems
    SET memory_used  = NEW.memory_used_kb,
        memory_total = NEW.memory_total_kb
    WHERE id = NEW.system_id;
    RETURN NEW;
END;
$$;
ALTER FUNCTION public.update_memory() OWNER TO postgres;


CREATE TRIGGER "metrics_cpu_usage_trigger"
    AFTER INSERT
    ON "metrics"
    FOR EACH ROW
    WHEN ((new.cpu_usage IS NOT NULL))
EXECUTE FUNCTION public.update_latest_cpu_usage();

CREATE TRIGGER "metrics_lastseen_trigger"
    AFTER INSERT
    ON "metrics"
    FOR EACH ROW
    WHEN ((new.system_id IS NOT NULL))
EXECUTE FUNCTION public.update_latest_lastseen();

CREATE TRIGGER "metrics_uptime_trigger"
    AFTER INSERT
    ON "metrics"
    FOR EACH ROW
    WHEN ((new.uptime IS NOT NULL))
EXECUTE FUNCTION public.update_latest_uptime();

CREATE TRIGGER "metrics_memory_trigger"
    AFTER INSERT
    ON "metrics"
    FOR EACH ROW
    WHEN ((new.memory_used_kb IS NOT NULL) AND (new.memory_total_kb IS NOT NULL))
EXECUTE FUNCTION public.update_memory();

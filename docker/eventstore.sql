CREATE EXTENSION IF NOT EXISTS semver;

CREATE SCHEMA IF NOT EXISTS registry;
CREATE TABLE IF NOT EXISTS registry.modules (
  name     VARCHAR(1000)  NOT NULL,
  version  SEMVER         NOT NULL,
  schema   JSONB          NOT NULL,
  module   BYTEA          NOT NULL,
  CONSTRAINT pk_registry PRIMARY KEY (name, version)
);

CREATE USER thalo_runtime WITH PASSWORD 'password';

GRANT ALL PRIVILEGES ON SCHEMA registry TO thalo_runtime;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA registry TO thalo_runtime;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA registry TO thalo_runtime;

GRANT ALL PRIVILEGES ON SCHEMA message_store TO thalo_runtime;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA message_store TO thalo_runtime;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA message_store TO thalo_runtime;

ALTER ROLE thalo_runtime SET search_path = '$user',public,message_store,registry;


{ pkgs }:

pkgs.writeShellScriptBin "init" ''
  #!/usr/bin/env bash

  DB_USER="''${APP_DATABASE__USER:=postgres}"
  DB_PASSWORD="''${APP_DATABASE__PASSWORD:=password}"
  DB_NAME="''${APP_DATABASE__NAME:=bulletin}"
  DB_PORT="''${APP_DATABASE__PORT:=5432}"
  DB_HOST="''${APP_DATABASE__HOST:=localhost}"

  if [ -z "''${SKIP_DOCKER}" ]
  then
    ${pkgs.docker-client}/bin/docker \
      -H ssh://sao \
      run \
      -e POSTGRES_USER="''${DB_USER}" \
      -e POSTGRES_PASSWORD="''${DB_PASSWORD}" \
      -e POSTGRES_DB="''${DB_NAME}" \
      -p "''${DB_PORT}":5432 \
      -d postgres \
      postgres -N 1000
  fi

  export PGPASSWORD="''${DB_PASSWORD}"

  until ${pkgs.postgresql_17_jit}/bin/psql -h "''${DB_HOST}" -U "''${DB_USER}" -p "''${DB_PORT}" -d "postgres" -c '\q';
  do
    echo "Postgres is still unavailable - sleeping"
    sleep 1
  done

  echo "Postgres is up and running on ''${DB_HOST}:''${DB_PORT} - running migrations now!"

  DATABASE_URL=postgres://''${DB_USER}:''${DB_PASSWORD}@''${DB_HOST}:''${DB_PORT}/''${DB_NAME}
  export DATABASE_URL
  ${pkgs.sqlx-cli}/bin/sqlx database create
  ${pkgs.sqlx-cli}/bin/sqlx migrate run

  echo "Postgres has been migrated, ready to go!"
''

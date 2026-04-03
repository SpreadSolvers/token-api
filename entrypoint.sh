#!/bin/sh
set -e
cd /app
/usr/local/bin/diesel migration run --config-file /app/diesel.docker.toml
exec /usr/local/bin/token-api

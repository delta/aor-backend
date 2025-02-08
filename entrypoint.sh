#!/bin/bash

service cron start & tail -f /var/log/cron.log
until nc -z -v -w30 db 5432
do
  echo "Waiting for database connection..."
  sleep 1
done

if [ "${PRODUCTION}" == "true" ]; then
  cargo run --release
else
  cargo watch -i logs -x run
fi

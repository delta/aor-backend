FROM rust:1.82.0-slim AS base
WORKDIR /usr/src/aot-backend

# Add PostgreSQL repository
RUN apt-get update -y && apt-get install -y \
    wget \
    gnupg2 \
    lsb-release \
 && echo "deb http://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" \
    | tee /etc/apt/sources.list.d/pgdg.list \
 && wget -qO - https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add - \
 && apt-get update -y \
 && apt-get install -y \
    libpq-dev \
    netcat-traditional \
    pkg-config \
    libssl-dev \
    cron \
    postgresql-client-17 

RUN cargo install diesel_cli --no-default-features --features postgres
RUN cargo install cargo-watch --locked
RUN cargo install cargo-chef

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base
COPY --from=planner /usr/src/aot-backend/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json
COPY . .
RUN cargo build


COPY crontab /etc/cron.d/backup-cron

RUN chmod 0644 /etc/cron.d/backup-cron

RUN crontab /etc/cron.d/backup-cron

RUN touch /var/log/cron.log

CMD ["cron", "-f", "-d", "8"]
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef

ARG SERVICE_NAME

WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin $SERVICE_NAME

FROM gcr.io/distroless/cc-debian12 AS runtime

ARG SERVICE_NAME

WORKDIR /app

EXPOSE 8080:8080
COPY --from=builder /app/target/release/$SERVICE_NAME $SERVICE_NAME
COPY configuration configuration
ENV APP_ENVIRONMENT=production
ENTRYPOINT ["./bulletin"]

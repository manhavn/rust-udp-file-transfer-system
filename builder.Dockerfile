FROM rust:alpine AS build
RUN apk add --no-cache musl-dev build-base

FROM alpine:latest AS runtime
RUN apk add --no-cache ca-certificates

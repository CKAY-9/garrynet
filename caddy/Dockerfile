# syntax=docker/dockerfile:1
# Caddy with the `rate_limit` directive (github.com/mholt/caddy-ratelimit), which the stock
# image doesn't include. The Caddyfile is mounted by docker-compose.yml, not baked in.

FROM caddy:2-builder-alpine AS builder
RUN xcaddy build --with github.com/mholt/caddy-ratelimit

FROM caddy:2-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy

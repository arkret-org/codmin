#!/bin/sh
set -eu

COLAND_URL="${COLAND_URL:-}"
COAUTH_URL="${COAUTH_URL:-}"
COAUTH_PUBLIC_URL="${COAUTH_PUBLIC_URL:-}"
CODMIN_TELEMETRY_ENDPOINT="${CODMIN_TELEMETRY_ENDPOINT:-}"
CODMIN_PORT="${CODMIN_PORT:-80}"
CODMIN_OAUTH_CLIENT_ID="${CODMIN_OAUTH_CLIENT_ID:-01GFWR28C4KNE04WG3HKXB7C9R}"
CODMIN_UPSTREAM_CA_FILE="${CODMIN_UPSTREAM_CA_FILE:-/etc/ssl/certs/ca-certificates.crt}"

RESOLVERS="$(awk '/^nameserver / { print $2 }' /etc/resolv.conf | paste -sd ' ' -)"
LOOKUP_UNAVAILABLE=0

if [ -z "$COLAND_URL" ]; then
    echo "COLAND_URL must be set" >&2
    exit 1
fi

extract_host() {
    url="$1"
    rest="${url#*://}"
    authority="${rest%%/*}"
    printf '%s\n' "${authority%%:*}"
}

can_resolve_url_host() {
    host="$(extract_host "$1")"
    [ -n "$host" ] || return 1
    [ "$LOOKUP_UNAVAILABLE" -eq 0 ] || return 1

    if timeout 0.5 getent hosts "$host" >/dev/null 2>&1; then
        return 0
    fi

    LOOKUP_UNAVAILABLE=1
    return 1
}

write_proxy_location() {
    location_path="$1"
    target_url="$2"
    auth_header="${3:-}"

    cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ${location_path} {
        proxy_pass ${target_url};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header X-Forwarded-Host \$host;
EOF

    if [ -n "$auth_header" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
        proxy_set_header Authorization ${auth_header};
EOF
    fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    }
EOF
}

write_dynamic_proxy_location() {
    location_path="$1"
    variable_name="$2"
    auth_header="${3:-}"

    cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ${location_path} {
        proxy_pass \$${variable_name};
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_set_header X-Forwarded-Host \$host;
EOF

    if [ -n "$auth_header" ]; then
cat >> /etc/nginx/conf.d/default.conf <<EOF
        proxy_set_header Authorization ${auth_header};
EOF
    fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    }
EOF
}

# Emit /config.json consumed by the Dioxus runtime. The value is
# JSON-escaped (backslash + double-quote) so an operator-supplied URL with
# a quote/backslash can't produce invalid JSON that wedges the SPA into the
# config-error panel.
json_escape() {
    # Escape backslash first, then double-quote.
    printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'
}
printf '{"coauth_public_url":"%s","telemetry_endpoint":"%s","oauth_client_id":"%s"}' \
    "$(json_escape "$COAUTH_PUBLIC_URL")" \
    "$(json_escape "$CODMIN_TELEMETRY_ENDPOINT")" \
    "$(json_escape "$CODMIN_OAUTH_CLIENT_ID")" \
    > /runtime/config.json

cat > /etc/nginx/conf.d/default.conf <<EOF
server {
    listen ${CODMIN_PORT};
    server_name _;
    root /usr/share/nginx/html;
    index index.html;

    # Validate HTTPS upstreams against the system CA bundle or an explicit
    # deployment CA. SNI and hostname verification remain enabled.
    proxy_ssl_verify on;
    proxy_ssl_server_name on;
    proxy_ssl_trusted_certificate ${CODMIN_UPSTREAM_CA_FILE};

    gzip on;
    gzip_min_length 1024;
    gzip_types application/wasm application/javascript text/css application/json image/svg+xml;

    # Hardening headers. CSP intentionally omits 'unsafe-inline' on
    # script-src; 'wasm-unsafe-eval' is required for the Dioxus WASM
    # bundle. Adjust connect-src if the deployment fronts additional
    # services beyond the same-origin proxy paths below.
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests" always;
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "no-referrer" always;
    add_header Permissions-Policy "geolocation=(), microphone=(), camera=(), payment=()" always;

    location = /config.json {
        alias /runtime/config.json;
        add_header Cache-Control "no-store" always;
    }

    # Healthz actually serves index.html so liveness fails (503) when the
    # wasm bundle disappears from disk, instead of nginx happily returning
    # a static 'ok' over an empty document root. wget --spider only checks
    # the status line so the response body is not transferred.
    location = /healthz {
        access_log off;
        try_files /index.html =503;
    }

    # Deep healthcheck that also verifies upstream coland
    # connectivity. Container orchestrators (k8s, nomad) can use this
    # as a readiness probe so a codmin pod is not advertised "ready"
    # while coland is unreachable. Errors (DNS failure, network down,
    # coland returning anything other than 2xx/3xx) return 503 via
    # nginx error_page so the probe fails as expected.
    location = /healthz/deep {
        access_log off;
        proxy_pass ${COLAND_URL}/healthz;
        proxy_set_header Host \$host;
        proxy_connect_timeout 2s;
        proxy_read_timeout 2s;
        proxy_intercept_errors on;
        error_page 500 502 503 504 =503 /healthz-upstream-down;
    }
    location = /healthz-upstream-down {
        access_log off;
        internal;
        return 503 "upstream coland unreachable\n";
    }
EOF

if [ -n "$COAUTH_URL" ]; then
    # coauth owns its own admin namespace `/_coauth/admin/*` (renamed from
    # `/_arkret/local/admin` in coauth bc06024); a single `/_coauth/`
    # location forwards the whole surface (proxy_pass passes the path
    # through unchanged). OAuth endpoints are `/oauth/token`, `/oauth/revoke`
    # (no `oauth2` prefix, no separate refresh route — refresh reuses the
    # token endpoint with grant_type=refresh_token). Auth / session
    # endpoints also live under the `gate` trust circle (`/_arkret/gate/...`),
    # per arkret-spec service-http-binding.md §2.2.
    if can_resolve_url_host "$COAUTH_URL"; then
        write_proxy_location "/auth/" "$COAUTH_URL"
        write_proxy_location "/_arkret/gate/" "$COAUTH_URL"
        write_proxy_location "/_coauth/" "$COAUTH_URL" "\$http_authorization"
        write_proxy_location "/authorize" "$COAUTH_URL"
        write_proxy_location "/oauth/" "$COAUTH_URL"
        write_proxy_location "/.well-known/" "$COAUTH_URL"
    else
        [ -n "$RESOLVERS" ] || RESOLVERS="127.0.0.11"
        cat >> /etc/nginx/conf.d/default.conf <<EOF
    resolver ${RESOLVERS} valid=30s ipv6=off;
    set \$coauth_backend ${COAUTH_URL};
EOF
        write_dynamic_proxy_location "/auth/" "coauth_backend"
        write_dynamic_proxy_location "/_arkret/gate/" "coauth_backend"
        write_dynamic_proxy_location "/_coauth/" "coauth_backend" "\$http_authorization"
        write_dynamic_proxy_location "/authorize" "coauth_backend"
        write_dynamic_proxy_location "/oauth/" "coauth_backend"
        write_dynamic_proxy_location "/.well-known/" "coauth_backend"
    fi
fi

if can_resolve_url_host "$COLAND_URL"; then
    write_proxy_location "/_arkret/self/events/" "$COLAND_URL"
    write_proxy_location "/_arkret/self/sync/" "$COLAND_URL"
    write_proxy_location "/_arkret/find/directory/" "$COLAND_URL"
    # `gate/account/*` is a shared auth/admission surface (service-http-binding
    # §2.1: served by clients AND the Station — the path segment is an
    # attack-surface classifier, NOT a single backing service). coauth owns most
    # of it (session-grants issue/refresh/logout/introspect, password-recovery,
    # email-auth, passkey/oidc, register) via the `/_arkret/gate/` rule above,
    # but `ak.gate.account.command.logout.v1` is a Station op (revoke
    # bearer + device session record + to-device) that coauth does not serve.
    # Route that one longer prefix to coland; nginx longest-prefix match makes
    # it win over `/_arkret/gate/` regardless of declaration order.
    write_proxy_location "/_arkret/gate/account/logout" "$COLAND_URL"
    # All other `/_arkret/*` trust-circle traffic (self/root/find/peer/open/edge)
    # terminates on coland, per arkret-spec service-http-binding.md §2.2. The
    # `/_arkret/gate/` coauth override above is a longer prefix and wins for
    # auth endpoints.
    write_proxy_location "/_arkret/" "$COLAND_URL"
    # coland's deployment-local operator + product surface (`/_coland/*`):
    # admin operator API plus the product-surface self/root extensions
    # (e.g. /_coland/self/circles, /_coland/root/identity/recovery-*). coauth
    # keeps its own `/_coauth/` namespace declared above.
    write_proxy_location "/_coland/" "$COLAND_URL"
else
    if ! grep -q "resolver " /etc/nginx/conf.d/default.conf; then
        [ -n "$RESOLVERS" ] || RESOLVERS="127.0.0.11"
        cat >> /etc/nginx/conf.d/default.conf <<EOF
    resolver ${RESOLVERS} valid=30s ipv6=off;
EOF
    fi
    cat >> /etc/nginx/conf.d/default.conf <<EOF
    set \$coland_backend ${COLAND_URL};
EOF
    write_dynamic_proxy_location "/_arkret/self/events/" "coland_backend"
    write_dynamic_proxy_location "/_arkret/self/sync/" "coland_backend"
    write_dynamic_proxy_location "/_arkret/find/directory/" "coland_backend"
    # Station device logout (see the resolvable branch above): a
    # longer prefix than `/_arkret/gate/` so it wins to coland.
    write_dynamic_proxy_location "/_arkret/gate/account/logout" "coland_backend"
    # All other `/_arkret/*` trust-circle traffic terminates on coland
    # (see the resolvable branch above for rationale).
    write_dynamic_proxy_location "/_arkret/" "coland_backend"
    # coland operator + product surface at `/_coland/*`.
    write_dynamic_proxy_location "/_coland/" "coland_backend"
fi

cat >> /etc/nginx/conf.d/default.conf <<EOF
    location ~* \.(wasm|js|css|png|jpg|ico|svg)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        # nginx drops ALL inherited add_header directives once any
        # add_header appears at this level, so the server-level security
        # headers must be repeated here. nosniff in particular is per-
        # response and is the relevant protection for script/wasm assets.
        add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'; upgrade-insecure-requests" always;
        add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;
        add_header X-Content-Type-Options "nosniff" always;
        add_header X-Frame-Options "DENY" always;
        add_header Referrer-Policy "no-referrer" always;
        add_header Permissions-Policy "geolocation=(), microphone=(), camera=(), payment=()" always;
    }

    location / {
        try_files \$uri \$uri/ /index.html;
    }
}
EOF

nginx -t
exec nginx -g "daemon off;"

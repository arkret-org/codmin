# codmin Admin Onboarding

This guide walks a new Arkret server administrator from a clean
deployment to a working codmin session with the right capabilities to
do real work. The strand is:

1. **DID setup** — the admin's own controller DID is bound to a coauth account.
2. **SSO config** — coauth knows the upstream identity provider so the admin can sign in.
3. **First login** — the admin opens codmin in a browser and completes the OAuth2 PKCE strand.
4. **Capability grant strand** — coauth issues the `ak.*` admin scopes the admin needs to drive codmin.

Until an admin has both a bound DID **and** the right capability
grants, codmin will surface every destructive action with a "not in
your grant list" hint via the `GrantedCapabilitiesView` component,
and the backend will reject any forged request with
`ak.error.capability_denied`.

## 1. Admin DID Setup

Each administrator is identified by a Decentralized Identifier (DID)
under the grammar `^did:[a-z0-9]+:[^\s]+$`. Method segments
MUST be lowercase ASCII alphanumeric — no `.`/`-`/`_`/`:`.

Examples that pass the SDK normalizer:

* `did:webvh:auth.example.net:admin-1`
* `did:webvh:admin.example.org:ops`

Always use `did:webvh` (the only DID method accepted by default). The
admin generates the keypair on a secure host and publishes the
`did:webvh` document — with its verifiable history log — at the
location implied by the method.

> `did:web` is **not** a default and **never** a fallback. It is only
> permitted for explicit no-history `service` / `personal_node` profiles
> where the deployment has deliberately opted out of webvh history; it
> MUST NOT be used for administrator or agent identities.

<!-- TODO(screenshot): /agents/personal/new — provision wizard step 1 showing the controller DID field with inline ValidatedInput error -->

Once the DID is published, bind it to the admin's coauth account via
the **Account Detail → Managed DID Bindings → Add binding** form on
`/coauth/accounts/:account_id`. The `control_proof` field is a
base64url-encoded signature challenge — the exact shape depends on
the DID method; see the coauth admin handler docs for the per-method
contract.

## 2. SSO Configuration

codmin itself does not perform authentication: it redirects every
unauthenticated user to coauth, which handles the OAuth2 PKCE
authorization code strand against the configured upstream provider.

Configure the upstream provider on coauth via **Upstream Providers**
(`/coauth/upstream-providers`):

1. Click **Add provider**.
2. Pick the kind — typically `oidc` for generic OIDC IdPs.
3. Fill in the issuer URL, client id, and client secret.
4. Save.

coauth will dynamically register the redirect URI back to codmin's
`/oauth/callback` route. Verify the provider appears in
`/coauth/connector-health` as **Healthy** before asking the new admin
to log in — a degraded connector will surface a confusing error on
first sign-in.

<!-- TODO(screenshot): /coauth/upstream-providers — list view with one Healthy upstream provider -->

## 3. First Login

The admin opens the codmin URL in a browser. The strand is:

1. codmin SPA loads, reads `/config.json`, learns the `coauth_public_url`.
2. SPA detects no session and redirects to `/login`.
3. `/login` page renders **Sign in with SSO** — clicking it kicks off the OAuth2 PKCE strand against coauth.
4. coauth redirects to the upstream IdP, which authenticates the user and redirects back to coauth.
5. coauth mints an authorization code, redirects to codmin's `/oauth/callback`.
6. codmin exchanges the code for an httpOnly session cookie via coauth.
7. SPA navigates to `/dashboard`.

If the admin's account is not marked `is_admin: true` on coauth, the
SPA renders **This account is not a server administrator** and
refuses to load the admin surface. Mark the account admin via the
**Coauth Accounts** page or the coauth admin CLI before retrying.

<!-- TODO(screenshot): /login — sign-in landing page with the SSO button and the auth status panel -->

## 4. Capability Grant Strand

Being signed in as an admin does NOT, by itself, grant the right to
drive every admin operation. The Arkret model is capability-based:
each admin action is gated by a specific `ak.*` capability scope,
and the operator must hold that scope (or a covering parent scope)
before the backend will accept the request.

For codmin administration the recommended starter grant set is:

| Capability | Why |
| --- | --- |
| `ak.agent.manage` | Meta-scope for the 11 personal-agent admin endpoints. |
| `ak.circle.create` / `ak.circle.manage` | Create and administer Circles. |
| `ak.realm.admin` | Destroy / classify Realms. |
| `ak.audit.read` | Read the audit log on `/audit`. |

Grant capabilities via **Capabilities** (`/capabilities`) — click
**Grant capability**, fill in the grantee DID (the admin's bound
DID, not the coauth `sub`), pick the scope, and submit. The grant
is published as a Move and becomes visible to the admin on their
next page load.

<!-- TODO(screenshot): /capabilities — grant capability dialog with the DID-validated input on the grantee DID field -->

Verify the grants landed on the admin's account by opening any
destructive form (e.g. **Provision new agent** on `/agents/personal`).
The `GrantedCapabilitiesView` panel at the top of the wizard will
list the operator's current grants and highlight whether the action's
required capability is present. If not, the **Submit** click will
fail with `ak.error.capability_denied` — request the missing grant
from a higher-privileged admin or via the coauth bootstrap migration.

## Troubleshooting

* **403 on every admin call** — capability grant missing; check `/capabilities`.
* **Session expired immediately after sign-in** — cookie domain mismatch between codmin and coauth; check `COAUTH_PUBLIC_URL` in `docker-entrypoint.sh`.
* **`/healthz/deep` returns 503** — coland is unreachable from codmin; check `COLAND_URL` and the network policy between the two containers.
* **`ak.error.capability_denied` on a button that's visible** — UI hiding is cosmetic only; the backend is the source of truth. Request the grant.

For deployment topology and the full port / path routing table see
[DEPLOYMENT.md](../DEPLOYMENT.md).

## Admin flowcharts

The flowcharts below are the canonical reference for the three most common
operational lifecycle strands admins drive from codmin. They are intentionally
text-only (ASCII step lists) so they survive in any review tool and can be
read on-call without rendering.

### Agent lifecycle: provisioning → pause → resume → deactivate

```text
[provision]
    │
    │  /agents/personal → "Provision new agent" wizard
    │  - DID method is did:webvh (the only accepted method)
    │  - generate key pair (browser-side; private key never leaves the wizard)
    │  - paste accountable_principal_ids chain (defaults to the operator's own grant)
    │  - submit
    │     → POST ak.gate.account.command.pair_agent_key.v1 to coauth
    │     → coland materializes the agent_state cell (Active)
    ▼
[Active]   (agent serves traffic; appears in /agents/personal listing)
    │
    │  Need a maintenance window?
    ▼
[pause]
    │  /agents/personal → row action "Pause"
    │     → POST /agents/{id}/pause to coland
    │     → audit row written: kind=agent.pause
    │     → coland_agent_state_cell flips to Paused
    │  Writes against the agent now reject with `agent_paused`.
    │  Reads remain allowed.
    ▼
[Paused]
    │
    │  Resume? (revert)                Deactivate? (terminal)
    ▼                                  ▼
[resume]                          [deactivate]
    │ /agents/personal               │ /agents/personal
    │   → row action "Resume"        │   → row action "Deactivate"
    │     → POST .../resume          │     → POST .../deactivate
    │     → cell back to Active      │     → cell to Deactivated (terminal)
    │                                │  All writes return agent_deactivated.
    │                                │  Inert reads remain allowed.
    ▼                                ▼
[Active]                          [Deactivated]
                                      │
                                      ▼
                                    no further transitions
                                    (bind a new agent under a new DID)
```

Notes:

- The historical `/agents/{id}/revoke` path is gone. Any "Revoke"
  button in older builds of codmin must be replaced with "Deactivate".
- Pause is reversible; Deactivate is not. The wizard requires a typed-out
  confirmation phrase before deactivate to prevent fat-fingering.

### Recovery policy rotation

```text
[review current policy]
    │  /policies/recovery → opens the principal's policy_version stack
    │  - read the active version (highest policy_version)
    │  - confirm proof_kinds set is current
    │  - inspect the witness inventory (DeviceQuorum, RecoveryUnlock,
    │    TrustedRecoveryService, PrincipalSigning rows)
    ▼
[draft new policy]
    │  "New revision" button
    │  - new policy_version = prev + 1 (form pre-fills)
    │  - edit proof_kinds (add / remove witness types)
    │  - edit body fields (threshold, witness set, freshness window)
    │  - save draft (not yet submitted)
    ▼
[validate draft]
    │  - codmin runs the local validator against
    │    recovery-policy.schema.json
    │  - flags any schema-mismatch fields red
    │  - if green, "Submit" enables
    ▼
[submit]
    │  - POST ak.recovery.policy.create to coland
    │  - coland writes the policy row + audit
    │  - policy stack now shows the new version as active
    ▼
[verify with a drill]
    │  - "Run drill" button on the new policy version
    │  - codmin opens a synthetic recovery_session against the new policy
    │  - drill completes → emits RecoveryReceipt
    │  - verify proof_summary matches expectations
    ▼
[done]
    Old policy versions remain in history (append-only). If the new
    policy needs rollback, draft *another* new revision; do not edit
    the past.
```

### Realm `media_service.foci[]` configuration

```text
[open realm config]
    │  /realms/{id}/media_service
    │  - view current foci[] (one row per focus)
    │  - for each focus: focus_id, backend, connect_url, issuer_kid, status
    ▼
[add a focus]
    │  "Add focus" button
    │  - choose backend (livekit / mediasoup / janus / arkret_native / moq_relay)
    │  - paste connect_url (the SFU/relay control endpoint)
    │  - choose issuer_kid (from coland's active kid set)
    │  - codmin synthesizes focus_id using the canonical rules:
    │      <backend>:<region>:<disambiguator>
    │  - validate (region matches [a-z0-9-]+, length checks)
    │  - save draft
    ▼
[stage]
    │  - new focus row is added with status=staged
    │  - clients with the matching profile can negotiate, but the realm
    │    won't auto-prefer it until you flip to active
    ▼
[health check]
    │  - "Probe" button issues a one-shot synthetic call against the focus
    │  - probe success → backend reachable, issuer_kid signs cleanly
    │  - probe failure → check connect_url and the issuer_kid binding
    ▼
[promote to active]
    │  - "Promote to active" flips status=staged → status=active
    │  - new tokens may now bind to this focus
    ▼
[(optional) demote / remove]
    │  - "Demote" flips active → drained; existing tokens valid till TTL
    │  - after >10min (max token TTL), "Remove" deletes the focus row
    ▼
[done]
    Audit rows under `realm.media_service.foci.update` capture every
    transition.
```

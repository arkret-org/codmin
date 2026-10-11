# Operator Runbook

This runbook covers operational tasks the codmin operator drives that have
**non-trivial blast radius**. The single biggest one is flipping a realm
into the strict-reject accountability posture (the
`ak.profile.accountable_principals.strict_reject.v1` profile).

For onboarding strands, the agent lifecycle, recovery rotation, and the
media-service config UI, see [`admin-onboarding.md`](./admin-onboarding.md).

## Flipping `accountable_principals.strict_reject` safely

Strict-reject is a realm-scoped profile that converts soft accountability
warnings into hard rejects across the entire `accountable_principal_ids` chain. The
companion narratives are:

- coauth side: [`coauth/docs/en/topics/deployment_hardening.md`](../../coauth/docs/en/topics/deployment_hardening.md)
- coland side: [`coland/docs/runbook.md` → Strict-reject profile toggle](../../coland/docs/runbook.md#strict-reject-profile-toggle-cxprofileaccountable_principalsstrict_rejectv1)

The codmin operator is the **canonical actor** of the flip; this runbook is
the operational checklist for the human at the keyboard.

### Pre-flip checklist (24 hours before)

1. **Snapshot accountability traffic.** Run the
   `accountability-claim-baseline` report from `/insights/accountability`.
   Capture:
   - Total accountability claims/sec.
   - Stale-vs-fresh ratio. Decision rule: if **stale ratio > 5%**, abort
     the flip and file bugs against the noisiest peer realms instead.
   - Top 10 noisiest peers by stale-claim volume.
2. **Pre-notify federation peers.** Send the templated "strict-reject
   cutover" message from `/notices/peer-coordination` to every peer realm
   listed in the baseline. Hold for written acknowledgement; do not flip
   without it for any peer >10% of inbound traffic.
3. **Silence the 4xx-ratio alert** for a 60-minute window centered on
   the planned cutover instant. Use `/alerts/silences/new` with the
   silence reason set to "strict-reject cutover".
4. **Confirm rollback path.** Read the rollback section below; rehearse
   the operator action in a non-prod realm if you haven't done this flip
   before.

### Flip procedure

1. Navigate to `/realms/{id}/profiles`.
2. Click **Edit profile set**.
3. Add `ak.profile.accountable_principals.strict_reject.v1` to the realm's
   declared profile set.
4. The form prompts for a **justification** (free-text, 32–512 chars) —
   this lands in the audit row. Write something a future ops engineer
   could read cold: "Q2 2026 SOC2 audit — switching to hard rejects per
   compliance recommendation". Do NOT write "as discussed in standup".
5. The form prompts for **confirmation phrase**: type the realm short
   id. This is the fat-finger guard.
6. Submit.

The submission produces three artifacts:

- A `ak.realm.profile.update` event in coland.
- A `profile.accountable_principals.strict_reject.flip` audit row with
  `direction = on`, the operator's actor_id, the justification, and the
  prior/new state digests.
- An `op:flip` row in codmin's own action log.

### Watch window (30 min post-flip)

Monitor these dashboards in `/monitoring/strict-reject-flip`:

| Metric | Expected behavior | Red flag |
|---|---|---|
| `coauth_accountable_principals_reject_total{profile="strict"}` | Rises from 0, levels off within (stale-baseline + 20%) | Continues climbing after 10 min; alert |
| `coauth_session_grant_failure_total{reason="agent_paused"|"agent_deactivated"|"accountability_grant_missing"}` | Flat at pre-flip baseline | Spike — indicates a deeper failure that strict-reject is now surfacing |
| `floria_delivery_profile_total{reason="accountability_reject"}` | Tiny spike, then flat | Sustained climb |
| `coland_realm_profile_update_total{profile_id="strict_reject"}` | Increments once | Increments more than once — someone else is flipping; abort |

### Expected admin alerts

These alerts will fire and are **expected** for the first 30–60 minutes:

- `coauth-strict-reject-baseline-overrun` — fires if reject rate
  exceeds the (stale-baseline + 20%) threshold for >5 minutes. **If this
  fires beyond the silence window, rollback.**
- `peer-federation-reject-spike` — fires if any single federated peer
  produces >100 rejects/min sustained. Usually means the peer is still
  on the old accountability shape; file an immediate ticket and decide
  whether to wait or rollback.

These alerts should NOT fire:

- `coauth-session-grant-collapse` — if total session grants drop >25%
  vs baseline, the flip has cascaded into a user-visible outage.
  **Rollback immediately**.
- `coland-audit-append-failures` — strict-reject increases audit volume;
  if this alert fires, the audit table is wedged. **Rollback** and
  triage the audit table separately.

### Rollback procedure

If any red-flag condition above fires, or if you receive an explicit
rollback directive:

1. Navigate to `/realms/{id}/profiles`.
2. Click **Edit profile set**.
3. Remove `ak.profile.accountable_principals.strict_reject.v1` from the declared
   profile set.
4. Confirmation phrase: realm short id again.
5. Submit. The form does NOT require justification for rollback (so that
   you can rollback fast); the audit row records `direction = off` with
   `justification = "operator rollback"`.

Effects of rollback:

- coauth restores the default lenient posture within the
  `revocation_freshness_window` (default 60s, often pinned to 30s in
  strict-reject postures).
- In-flight rejects already audited remain audited; no new reject
  decisions fire after the mirror refresh.
- The 4xx ratio returns to pre-flip baseline within 2-3 minutes.

After rollback:

1. Capture a post-mortem in `/postmortems/new` referencing the flip and
   rollback audit rows.
2. File bugs against any peer that triggered the rollback.
3. Schedule a retry only after the root cause is addressed.

### Frequently observed misconceptions

- **"Strict-reject is global"** — No, it's realm-scoped. A single codmin
  deployment may simultaneously host realms in strict and lenient
  postures. The flip you perform applies only to the realm you selected.
- **"The flip is reversible by editing the audit row"** — No. Audit rows
  are append-only; rollback is by editing the realm's profile set and
  producing a new audit row in the opposite direction.
- **"Rollback erases the rejects that fired"** — No. Rejects already
  audited remain audited; they continue to appear in compliance reports
  for the audit-retention window (default 90 days).

## Other operator-driven strands

- **Recovery policy rotation** — see
  [`admin-onboarding.md` → Admin flowcharts → Recovery policy
  rotation](./admin-onboarding.md#recovery-policy-rotation).
- **Realm media_service foci configuration** — see
  [`admin-onboarding.md` → Admin flowcharts → Realm media_service
  foci[] configuration](./admin-onboarding.md#realm-media_servicefoci-configuration).
- **Agent lifecycle (pause / resume / deactivate)** — see
  [`admin-onboarding.md` → Admin flowcharts → Agent
  lifecycle](./admin-onboarding.md#agent-lifecycle-provisioning--pause--resume--deactivate).

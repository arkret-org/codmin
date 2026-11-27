# Security Policy

## Reporting a Vulnerability

Please report security vulnerabilities privately to **chris@acroidea.com** (PGP available on request).

- **Acknowledgement SLA**: 3 business days
- **Triage**: within 7 business days
- **Fix target**: 30 business days for high-severity issues; 90 days for medium/low

Do **not** open a public GitHub issue for security bugs.

## Scope

In-scope:
- Authentication / authorization bypass
- Information leakage (PII, scope membership, secrets)
- Cryptographic weaknesses
- Denial-of-service amplification

Out-of-scope:
- Self-hosted dev defaults intended to be overridden in production
- Theoretical issues without practical exploitation

## Administrative reads

Render protected records only after verifying the operator's current authorization
for the source Realm or Sidecar. A read projection's `effective_scope` must
match the creating Event's signed `scope_ref`.

Violations of these invariants are treated as security issues.

## Disclosure

Coordinated disclosure preferred; we will credit reporters in CHANGELOG unless anonymity is requested.

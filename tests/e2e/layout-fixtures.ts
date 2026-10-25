import type { Page } from '@playwright/test';

export const layoutIds = {
  account: '01JZ9PK6HKFY0MM7C0TMZ1X8N7',
  secondAccount: '01JZ9PK6HKFY0MM7C0TMZ1X8N8',
  actor: 'ak:did_core:web:alice.example',
  secondActor: 'ak:did_core:web:bob.example',
  station: 'ak:did_core:web:station.example',
  realm: 'ak:realm:AUwZORI60DxE1STM676ap_m1sIzBcRRCjoewWDPyAjx-',
  secondRealm: 'ak:realm:AWASxhHncyxoEbHwACOwwtlRkQ3MT2gAAiT3ZBg5sMbb',
  space: 'ak:space:AaRjkifqF_BCAMvNiejGgtBBGiw19WNl2qDz_8ro1FF3',
  secondSpace: 'ak:space:AVX7ebly5NnDPvZ6X2AdCALS2TlVGPEDv2mXBOJl2YKj',
  device: 'device-1234',
  session: 'session-1234',
  operation: 'ak:operation:0199a3a6-8000-7000-8000-000000000001',
  secondOperation: 'ak:operation:0199a3a6-8000-7000-8000-000000000002',
};

const created = '2026-10-12T08:30:00.000Z';
const recent = '2026-10-25T02:15:00.000Z';
const expires = '2026-11-25T08:30:00.000Z';

function resource(type: string, id: string, attributes: unknown, path: string) {
  return { type, id, attributes, links: { self: `${path}/${encodeURIComponent(id)}` } };
}
function single(data: unknown, path: string) { return { data, links: { self: path } }; }
function cursorPage(data: unknown[], path: string, next: string | null = null, count = data.length) {
  return { data, links: { self: path, next }, meta: { count } };
}
function collection(data: unknown[]) { return { data, total: data.length, next_cursor: null }; }


// Accepted capability rows follow coland-contracts' exact-current round-trip
// fixture: complete Actors, selectors and source revisions stay intact.
function capability(realmId: string, principalId: string, revoked = false) {
  const eventRef = realmId.replace('ak:realm:', 'ak:event:');
  const grantId = realmId.replace('ak:realm:', 'ak:grant:');
  const actor = { kind: 'account', account_id: { principal_id: principalId, station_id: layoutIds.station } };
  const issuer = { kind: 'account', account_id: { principal_id: layoutIds.actor, station_id: layoutIds.station } };
  const root = { kind: 'realm_root', realm_id: realmId, authority_event_ref: eventRef, authority_generation: 0 };
  return {
    selector: { kind: 'capability_grant', grant_id: grantId },
    source_stream_ref: { kind: 'realm', realm_id: realmId },
    revision: { commit_id: realmId.replace('ak:realm:', 'ak:realm_commit:'), stream_position: revoked ? 8 : 2 },
    value: {
      id: grantId, schema: 'ak.schema.capability.v1', realm_id: realmId,
      issuer_id: issuer, subject: actor, actions: ['ak.message.create'], resources: [{ kind: 'realm', realm_id: realmId }],
      issuer_authority_refs: [root], authority_depth: 1, authority_root_refs: [root],
      issued_at: created, status: revoked ? 'revoked' : 'active',
      ...(revoked ? { revoked_by: issuer, revoked_at: recent } : {}),
    },
  };
}

// The v1 backup registry has exactly one class (secret_storage). These are
// closed metadata summaries; ciphertext and envelope authentication are absent.
const backupSeries = 'ak:backup_series:01964137-1000-7000-8000-000000000000';
const backupDigest = 'sha256:709e80c88487a2411e1ee4dfb9f22a861492d20c4765150c0c794abd70f8147c';
const accountIdentity = { principal_id: layoutIds.actor, station_id: layoutIds.station };
const backupRows = [0, 1].map(index => ({
  backup_id: 'ak:backup:01964137-2000-7000-8000-00000000000' + (index + 1),
  actor_id: { kind: 'account', account_id: accountIdentity },
  backup_kind: 'secret_storage', backup_version: 'kb_1', series_id: backupSeries, series_seq: index,
  created_at: index ? recent : created, ciphertext_digest: backupDigest,
  encryption: { recipient_method: index ? 'recovery_public_key' : 'secret_storage_key', ...(index ? { recipient_key_ref: 'recovery-hpke-1' } : {}) },
  ...(index ? { supersedes_id: 'ak:backup:01964137-2000-7000-8000-000000000001', supersedes_digest: backupDigest, expires_at: expires } : {}),
}));
const policyRows = [
  { policy_id: 'ak:policy:01964137-3000-7000-8000-000000000002', account_id: accountIdentity, version: 2,
    acceptance_basis_ref: layoutIds.realm.replace('ak:realm:', 'ak:realm_commit:'), trust_domain: 'ak:trust_domain:station.example',
    supersedes_id: 'ak:policy:01964137-3000-7000-8000-000000000001', issued_at: recent, accepted_at: recent, expires_at: expires,
    methods: [{ kind: 'did_root' }, { kind: 'trusted_recovery_service', services: [{ service_id: 'ak:did_core:web:recovery.example', audience: 'https://recovery.example', authorization_verification_method: 'did:web:recovery.example#recovery-authorization' }] }] },
  { policy_id: 'ak:policy:01964137-3000-7000-8000-000000000001', account_id: accountIdentity, version: 1,
    acceptance_basis_ref: layoutIds.secondRealm.replace('ak:realm:', 'ak:realm_commit:'), trust_domain: 'ak:trust_domain:station.example',
    issued_at: created, expires_at: recent, methods: [] },
];
const organizationId = 'ak:did_core:web:engineering.example';
const partnerOrganizationId = 'ak:did_core:web:community.example';
function organizationControl(id: string, delegated = false) {
  const principalControlRealm = delegated ? layoutIds.secondRealm : layoutIds.realm;
  return {
    control: { id: delegated ? 'org-control-2' : 'org-control-1', organization_id: id, organization_did: id.replace('ak:did_core:', 'did:'),
      principal_control_realm_id: principalControlRealm, control_stream_ref: principalControlRealm.replace('ak:realm:', 'ak:event:'),
      pcr_commit_ref: principalControlRealm.replace('ak:realm:', 'ak:realm_commit:'),
      bootstrap_authorization: delegated ? 'delegated_governance' : 'did_controller_proof',
      ...(delegated ? { bootstrap_delegation_ref: 'organization-delegation-2', executed_by: layoutIds.actor } : {}),
      created_at: created, updated_at: recent },
    delegations: [{ id: delegated ? 'delegation-2' : 'delegation-1', delegation_ref: delegated ? 'organization-delegation-2' : 'organization-delegation-1',
      organization_id: id, delegate_id: 'ak:did_core:web:auth.example', issuer_role: 'account_authority',
      purposes: ['principal_control_realm_bootstrap'], covered_relationships: ['owner', 'sponsor'],
      covered_control_scopes: ['official_badge', 'directory_listing'], status: delegated ? 'revoked' : 'active',
      valid_from: created, valid_until: expires, created_by: layoutIds.actor, created_at: created, updated_at: recent,
      ...(delegated ? { revoked_at: recent } : {}) }],
  };
}

function account(id: string, name: string, status = 'active', principal = layoutIds.actor) {
  return resource('accounts', id, {
    handle: name.toLowerCase(), status, created_at: created, updated_at: recent,
    locked_at: status === 'locked' ? recent : null, deactivated_at: null,
    admin: name === 'Alice', display_name: name, avatar_url: null,
    preferred_locale: 'en', primary_principal_id: principal, principal_ids: [principal],
  }, '/_coauth/admin/accounts');
}
function personalSession(id: string, name: string, userId = layoutIds.account) {
  return resource('personal-session', id, {
    created_at: created, revoked_at: null, owner_user_id: userId, owner_client_id: null,
    actor_user_id: userId, human_name: name, scope: 'urn:arkret:admin:*',
    last_active_at: recent, last_active_ip: '192.0.2.42', expires_at: expires,
  }, '/_coauth/admin/personal-sessions');
}
function realm(id: string, title: string, encrypted: boolean) {
  return {
    kind: 'realm', id, strand: {},
    strand_id: 'ak:strand:AbZyWjHz86aXMxPVsi5SUW9QgFqny2SpKW_jkeUm61xz',
    realm_id: id, title, topic: encrypted ? 'Private engineering collaboration' : 'Public project announcements',
    category: 'community', realm_class: 'collaboration', discoverability: encrypted ? 'invite_only' : 'public',
    default_join_rule: encrypted ? 'invite' : 'public', tags: ['engineering', 'arkret'],
    public: !encrypted, member_count: encrypted ? 24 : 108,
    members: [layoutIds.actor, layoutIds.secondActor], created_by: layoutIds.actor,
    history_access: 'since_join', is_encrypted: encrypted, is_blocked: false,
    plaintext_visible_services: [], deleted: false, created_at: created, updated_at: recent,
  };
}

const describe = {
  service_id: layoutIds.station,
  service_resolution: { did: 'did:web:station.example', method_history_head: 'fixture-method-head', version_id: 'fixture-v1' },
  trust_domain: 'ak:trust_domain:station.example', service_kind: 'station', protocol_version: '1.0',
  supported_profiles: [], supported_operation_bundles: ['ak.operation_bundle.station.describe.v1'],
  transport_bindings: [{ kind: 'http_json', base_url: 'https://station.example/', extension_profile_required: null }],
  supported_features: [],
  auth_metadata: {
    account_authority: { origin: 'https://auth.example', gate_account_base_url: 'https://auth.example/_arkret/gate/account' },
    methods: [{ method: 'oidc', issuer_uri: 'https://auth.example', openid_configuration_url: 'https://auth.example/.well-known/openid-configuration', client_id: '01GFWR28C4KNE04WG3HKXB7C9R', scopes: ['openid', 'profile'], grant_exchange: { kind: 'account_handoff' } }],
    did_binding_methods: ['session_dpop'],
  },
  limits: {},
  rate_limit_policy: { policy_version: '1', entries: [] },
  plaintext_visibility: { max_visibility: 'private_plaintext', data_classes: ['message_content'] },
  verified_profiles: [], interop_surfaces: [{ name: 'federation.bridge', kind: 'external_interop' }],
  development_mode: false,
};

const bridge = {
  contract: 'arkret.rest.coauth_admin_bridge.v1', api_base_path: '/_coauth/admin',
  accounts_path: '/_coauth/admin/accounts', account_detail_path_template: '/_coauth/admin/accounts/{account_id}',
  account_dids_path_template: '/_coauth/admin/accounts/{account_id}/dids',
  account_claims_path_template: '/_coauth/admin/accounts/{account_id}/claims',
  risk_action_path_template: '/_coauth/admin/accounts/{account_id}/risk-action',
  risk_action_current_path_template: '/_coauth/admin/accounts/{account_id}/risk-action/current',
  risk_action_history_path_template: '/_coauth/admin/accounts/{account_id}/risk-action/history',
  risk_action_approve_path_template: '/_coauth/admin/accounts/{account_id}/risk-action/{proposal_id}/approve',
  risk_action_execute_path_template: '/_coauth/admin/accounts/{account_id}/risk-action/{proposal_id}/execute',
  risk_action_approval_mode: 'detached_ed25519_jws',
  risk_action_examples: {
    proposal_request: { action: 'lock', reason: 'Suspicious recovery activity', ticket: 'SEC-2048' },
    approve_request: { action: 'lock', ticket: 'SEC-2048', approved_by: layoutIds.actor, approval_note: 'Reviewed by on-call administrator', approval_proof_jws: 'protected..signature' },
    execute_request: { action: 'lock', ticket: 'SEC-2048', execution_note: 'Execute via controlled mutation worker' },
  },
};

const manifest = {
  contract: 'arkret.rest.integration_manifest.v1', service: 'coauth', service_kind: 'account_authority',
  api_base_path: '/_coauth', describe_path: '/_coauth/account/integration/describe',
  dependencies: [{ service: 'coland', purpose: 'station_session_exchange', required_contract: 'arkret.rest.principal_bridge.v1', discovery_path: '/_coauth/account/auth/bridge/describe', mode: 'remote_service_contract' }],
  surfaces: [
    { name: 'auth_bridge', method: 'GET', path: '/_coauth/account/auth/bridge/describe', contract: 'arkret.rest.auth_bridge.v1' },
    { name: 'admin_bridge', method: 'GET', path: '/_coauth/admin/bridge/describe', contract: bridge.contract },
  ],
};

/** Non-empty, contract-shaped data for layout acceptance; unsupported APIs fail visibly. */
export async function setupLayout(page: Page, { dark = false, language = 'en' }: { dark?: boolean; language?: string } = {}) {
  const errors: string[] = [];
  const requests: string[] = [];
  const missing: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error' && /^Glory\b|admin region render failed/.test(message.text())) errors.push(message.text());
  });
  await page.addInitScript(({ dark, language }) => {
    for (const [key, value] of Object.entries({
      session_active: '1', is_admin: 'true', language, theme: dark ? 'dark' : 'light',
      user_id: 'alice', display_name: 'Alice Chen', coauth_public_url: location.origin,
    })) localStorage.setItem(key, JSON.stringify(value));
  }, { dark, language });

  const accounts = [account(layoutIds.account, 'Alice'), account(layoutIds.secondAccount, 'Bob', 'locked', layoutIds.secondActor)];
  const devices = [
    { id: layoutIds.device, account_id: layoutIds.account, display_name: 'Alice’s workstation', risk_level: 'low', mfa_state: 'verified', registered_at: created, revoked_at: null as string | null },
    { id: 'device-5678', account_id: layoutIds.account, display_name: 'Recovery tablet', risk_level: 'high', mfa_state: 'required', registered_at: created, revoked_at: recent },
  ];
  const personal = [personalSession(layoutIds.session, 'Deployment automation'), personalSession('session-5678', 'Read-only monitoring', layoutIds.secondAccount)];
  const oauth = ['Desktop application', 'Operations dashboard'].map((name, index) => resource('oauth-session', `oauth-${index + 1}`, {
    created_at: created, finished_at: index ? recent : null, user_id: index ? layoutIds.secondAccount : layoutIds.account,
    user_session_id: `browser-${index + 1}`, client_id: `client-${index + 1}`, scope: 'openid profile urn:arkret:admin:*',
    user_agent: 'Layout acceptance browser', last_active_at: recent, last_active_ip: '192.0.2.42', human_name: name,
  }, '/_coauth/admin/oauth-sessions'));
  const providers = [
    resource('upstream-oauth-provider', 'provider-1234', { oidc_issuer_uri: 'https://login.example.org', human_name: 'Company identity', brand_name: 'Example', created_at: created, disabled_at: null, source: 'database' }, '/_coauth/admin/upstream-oauth-providers'),
    resource('upstream-oauth-provider', 'provider-5678', { oidc_issuer_uri: 'https://research.example.org', human_name: 'Research partner', brand_name: 'Research', created_at: created, disabled_at: recent, source: 'database' }, '/_coauth/admin/upstream-oauth-providers'),
  ];
  const links = [layoutIds.account, layoutIds.secondAccount].map((user, index) => resource('upstream-oauth-link', `link-${index + 1}`, {
    created_at: created, updated_at: recent, provider_id: index ? 'provider-5678' : 'provider-1234', subject: `subject-${index + 1}@example.org`, user_id: user, human_account_name: index ? 'Bob' : 'Alice',
  }, '/_coauth/admin/upstream-oauth-links'));
  const registration = [
    resource('user-registration-token', 'registration-1234', { token: 'invite-engineering-2026', valid: true, usage_limit: 25, times_used: 9, created_at: created, last_used_at: recent, expires_at: expires, revoked_at: null }, '/_coauth/admin/user-registration-tokens'),
    resource('user-registration-token', 'registration-5678', { token: 'invite-community-2026', valid: false, usage_limit: null, times_used: 42, created_at: created, last_used_at: recent, expires_at: null, revoked_at: recent }, '/_coauth/admin/user-registration-tokens'),
  ];
  const realms = [realm(layoutIds.realm, 'Engineering', true), realm(layoutIds.secondRealm, 'Community announcements', false)];
  const spaces = [
    { id: layoutIds.space, name: 'Engineering workspace', realm_id: layoutIds.realm, kind: 'list', member_count: 24, health: 'active', created_at: created, parent_space_id: null },
    { id: layoutIds.secondSpace, name: 'Release archive', realm_id: layoutIds.realm, kind: 'list', member_count: 12, health: 'archived', created_at: created, parent_space_id: layoutIds.space },
  ];
  const actors = [
    { id: layoutIds.actor, principal_id: layoutIds.actor, account_id: JSON.stringify({ principal_id: layoutIds.actor, station_id: layoutIds.station }), handle: 'alice', display_name: 'Alice Chen', status: 'active', is_admin: true, deactivation_federation_incomplete: null, deactivation_partial: null, created_at: created, last_active_at: recent, device_count: 3, realm_count: 4 },
    { id: layoutIds.secondActor, principal_id: layoutIds.secondActor, account_id: JSON.stringify({ principal_id: layoutIds.secondActor, station_id: layoutIds.station }), handle: 'bob', display_name: 'Bob Morgan', status: 'locked', is_admin: false, deactivation_federation_incomplete: null, deactivation_partial: null, created_at: created, last_active_at: recent, device_count: 1, realm_count: 2 },
  ];
  const media = [
    { kind: 'media', sha256: 'a'.repeat(64), media_type: 'image/png', filename: 'architecture-overview.png', realm_id: layoutIds.realm, encrypted: true, uploaded_by: layoutIds.actor, size_bytes: 1048576, created_at: created },
    { kind: 'media', sha256: 'b'.repeat(64), media_type: 'application/pdf', filename: 'community-handbook.pdf', realm_id: layoutIds.secondRealm, encrypted: false, uploaded_by: layoutIds.secondActor, size_bytes: 245760, created_at: recent },
  ];
  const mediaActors = [
    { actor_id: layoutIds.actor, display_name: 'Alice Chen', blob_count: 34, total_size: 10485760 },
    { actor_id: layoutIds.secondActor, display_name: 'Bob Morgan', blob_count: 12, total_size: 2457600 },
  ];
  const federation = [
    { kind: 'federation_operation', operation_id: layoutIds.operation, realm_id: layoutIds.realm, operation_kind: 'create', canonical_kind: 'ak.message.create', strand_id: 'ak:strand:AbZyWjHz86aXMxPVsi5SUW9QgFqny2SpKW_jkeUm61xz', track: 'main', digest: 'sha256:2d6f7a4067b2a0a7a4f36c55e82221f93fe36d33bdbb59a88d956e5ef572e2b0', created_at: created },
    { kind: 'federation_operation', operation_id: layoutIds.secondOperation, realm_id: layoutIds.secondRealm, operation_kind: 'update', canonical_kind: 'ak.message.revise', strand_id: null, track: null, digest: null, created_at: recent },
  ];
  const handles = [
    { id: 'handle-1234', canonical_uri: 'https://station.example/@alice', aliases: ['alice@station.example'], issuer_id: layoutIds.station, subject_id: layoutIds.actor, assigned_at: created, expires_at: expires, last_reassignment_at: null, status: 'active' },
    { id: 'handle-5678', canonical_uri: 'https://station.example/@bob', aliases: [], issuer_id: layoutIds.station, subject_id: layoutIds.secondActor, assigned_at: created, expires_at: null, last_reassignment_at: recent, status: 'revoked' },
  ];

  await page.route('**/*', async route => {
    const url = new URL(route.request().url());
    const path = decodeURIComponent(url.pathname);
    const method = route.request().method();
    if (path.startsWith('/_') || path === '/config.json' || path === '/health') requests.push(`${method} ${url.pathname}${url.search}`);
    const reply = (json: unknown) => route.fulfill({ json });
    if (path === '/config.json') return reply({ coauth_public_url: url.origin, oauth_client_id: '01GFWR28C4KNE04WG3HKXB7C9R' });
    if (path === '/_coland/admin/viewer') return reply({ principal_id: layoutIds.actor, state: 'active', is_server_admin: true });
    if (path === '/_arkret/describe') return reply(describe);
    if (path === '/_coland/admin/server/info') return reply({ server_version: '0.3.0', protocol_version: '1.0', server_name: 'Arkret Engineering', uptime: 86400, service_id: layoutIds.station, trust_domain: 'station.example', development_mode: false, allow_public_registration: false });
    if (path === '/_coland/admin/server/stats') return reply({ actor_count: 1284, active_actor_count: 1128, realm_count: 32, device_count: 2048, report_count: 3, federation_peer_count: 8, applet_count: 12, blob_count: 8492, blob_total_size: 7516192768, generated_at: recent });
    if (path === '/_coland/admin/server/status') return reply({ status: 'ok', service_id: layoutIds.station, storage: 'postgres', development_mode: false, checked_by: layoutIds.actor, generated_at: recent, counts: { accounts: 1284, devices: 2048, realms: 32 } });

    if (method === 'GET' && path === '/_coauth/admin/accounts') {
      const search = (url.searchParams.get('filter[search]') || '').toLowerCase();
      const rows = search ? accounts.filter(row => JSON.stringify(row.attributes).toLowerCase().includes(search)) : accounts;
      return reply(cursorPage(rows, path));
    }
    if (method === 'GET' && /^\/_coauth\/admin\/accounts\/[^/]+$/.test(path)) {
      const id = path.split('/').at(-1)!;
      return reply(single(accounts.find(row => row.id === id) || account(id, 'Alice'), path));
    }
    if (method === 'GET' && /^\/_coauth\/admin\/accounts\/[^/]+\/devices$/.test(path)) return reply({ data: devices });
    if (method === 'POST' && /\/devices\/[^/]+\/revoke$/.test(path)) {
      const target = devices.find(row => row.id === path.split('/').at(-2));
      if (target) target.revoked_at = recent;
      return reply({ device: target, revoked_session_grants: 2 });
    }
    if (method === 'GET' && /\/_coauth\/admin\/accounts\/[^/]+\/dids$/.test(path)) {
      const id = path.split('/')[4];
      const resolver = { mode: 'local_bindings', resolver: null, proof_required_for_pairwise: true };
      return reply({ data: [
        { id: 'binding-1', account_id: id, did: 'did:web:alice.example', kind: 'primary', state: 'active', primary: true, active: true, verification_status: 'verified', resolver, created_at: created, last_verified_at: recent, last_resolver_receipt_id: 'receipt-1', revoked_at: null },
        { id: 'binding-2', account_id: id, did: 'did:web:recovery.example', kind: 'recovery', state: 'pending_proof', primary: false, active: false, verification_status: 'pending', resolver, created_at: created, last_verified_at: null, last_resolver_receipt_id: null, revoked_at: null },
      ], meta: { resolver, supported_verification_methods: ['did_control'], supports_write_operations: true } });
    }
    if (method === 'GET' && /\/_coauth\/admin\/accounts\/[^/]+\/claims$/.test(path)) {
      return reply({ data: [
        { id: 'claim-1', account_id: layoutIds.account, claim_kind: 'email', value: 'alice@example.org', state: 'active', source: 'organization_directory', subject: layoutIds.actor, issuer: 'ak:did_core:web:auth.example', verifier_did: layoutIds.actor, represented_organization: 'Example Engineering', payload: {}, issued_at: created, expires_at: expires, revoked_at: null, revoked_reason: null },
        { id: 'claim-2', account_id: layoutIds.account, claim_kind: 'organization_role', value: 'on-call', state: 'revoked', source: 'administrator', subject: layoutIds.actor, issuer: 'ak:did_core:web:auth.example', verifier_did: layoutIds.actor, represented_organization: 'Example Engineering', payload: {}, issued_at: created, expires_at: null, revoked_at: recent, revoked_reason: 'Rotation completed' },
      ] });
    }
    if (method === 'GET' && /\/risk-action\/current$/.test(path)) {
      const current = { account_id: path.split('/')[4], state_record_id: 'risk-state-2049', proposal_id: 'proposal-2049', action: 'lock', lifecycle_state: 'proposed', last_operation: 'submit', transition_kind: 'propose', previous_state: 'idle', state_revision: 5, allowed_next_transitions: ['approve'], ticket: 'SEC-2049', recorded_at: recent, recorded_by: layoutIds.actor, recorded_by_handle: 'alice' };
      return reply(single(resource('account-risk-action-current', current.account_id, current, '/_coauth/admin/accounts'), path));
    }
    if (method === 'GET' && /\/risk-action\/history$/.test(path)) return reply({ data: [
      { account_id: layoutIds.account, state_record_id: 'risk-state-2048', proposal_id: 'proposal-2048', action: 'lock', transition_kind: 'execute', previous_state: 'approved', next_state: 'idle', state_revision: 4, ticket: 'SEC-2048', recorded_at: recent, recorded_by: layoutIds.actor, recorded_by_handle: 'alice', approval_note: 'Security review completed', execution_note: 'Recovery verified with account owner' },
    ] });
    if (path === '/_coauth/admin/bridge/describe') return reply(bridge);
    if (path === '/_coauth/account/integration/describe') return reply(manifest);
    if (path === '/_coauth/admin/personal-sessions' && method === 'GET') return reply(cursorPage(personal, path));
    if (path.endsWith('/regenerate') && path.startsWith('/_coauth/admin/personal-sessions/')) {
      const row = personal.find(row => row.id === path.split('/').at(-2)) || personal[0];
      return reply(single({ ...row, attributes: { ...row.attributes as object, access_token: 'layout-one-shot-token' } }, path));
    }
    if (path === '/_coauth/admin/oauth-sessions' && method === 'GET') return reply(cursorPage(oauth, path));
    if (path === '/_coauth/admin/upstream-oauth-providers' && method === 'GET') return reply(cursorPage(providers, path));
    if (path === '/_coauth/admin/upstream-oauth-links' && method === 'GET') return reply(cursorPage(links, path));
    if (path === '/_coauth/admin/user-registration-tokens' && method === 'GET') return reply(cursorPage(registration, path));
    if (path === '/_coauth/admin/notification-channels') return reply({ channels: [{ channel: 'Email', configured: true }, { channel: 'SMS', configured: false }, { channel: 'Webhook', configured: true }] });
    if (path === '/_coauth/admin/notification-templates') return reply({ templates: [{ key: 'account.verify_email', description: 'Confirm an email address before activating the account.' }, { key: 'account.recovery_started', description: 'Notify the account owner when a recovery flow begins.' }, { key: 'security.device_revoked', description: 'Confirm that a registered device and its sessions were revoked.' }] });
    if (path === '/_coauth/admin/connector-health') return reply({ providers: [{ provider: 'Company OIDC', account_id: layoutIds.account, status: 'healthy', error: null }, { provider: 'Research OIDC', account_id: layoutIds.secondAccount, status: 'unhealthy', error: 'Upstream discovery timed out; retry scheduled.' }] });
    if (path === '/_coauth/admin/audit-feed') return reply({ data: [
      { id: 'audit-2048', admin_user_id: layoutIds.account, operation: 'device.revoke', resource_kind: 'device', resource_id: layoutIds.device, details: { reason: 'SEC-2048 recovery verification', revoked_session_grants: 2 }, created_at: recent, signature_status: 'verified' },
      { id: 'audit-2047', admin_user_id: layoutIds.account, operation: 'upstream_provider.disable', resource_kind: 'upstream_oauth_provider', resource_id: 'provider-5678', details: { reason: 'Partner maintenance window' }, created_at: created, signature_status: 'unsigned' },
    ] });


    if (path === '/_coland/admin/capabilities' && method === 'GET') {
      const filters = Object.fromEntries([...url.searchParams].filter(([key]) => key.startsWith('filter[')).map(([key, value]) => [key.slice(7, -1), value]));
      let rows = [capability(layoutIds.realm, layoutIds.actor), capability(layoutIds.secondRealm, layoutIds.secondActor, true)];
      if (filters.state && filters.state !== 'all') rows = rows.filter(row => row.value.status === filters.state);
      if (filters.realm_id) rows = rows.filter(row => row.source_stream_ref.realm_id === filters.realm_id);
      if (filters.subject_actor_id) rows = rows.filter(row => JSON.stringify(row.value.subject) === JSON.stringify(JSON.parse(filters.subject_actor_id)));
      return reply({ capabilities: rows, total: rows.length, has_more: false, filters });
    }
    if (path === '/_coland/admin/key-backups' && method === 'GET') {
      const series = url.searchParams.get('series_id');
      const kind = url.searchParams.get('backup_kind');
      return reply({ backups: backupRows.filter(row => (!series || row.series_id === series) && (!kind || row.backup_kind === kind)),
        active_series: { account_id: accountIdentity, control_realm_id: layoutIds.realm,
          authority_commit_id: layoutIds.realm.replace('ak:realm:', 'ak:realm_commit:'),
          secret_storage: { state: 'active', active_series_id: backupSeries, series_pointer_version: 2 } }, has_more: false });
    }
    if (path === '/_coland/root/identity/recovery-policies' && method === 'GET') {
      const principal = url.searchParams.get('principal_id');
      return reply({ policies: policyRows.filter(row => !principal || row.account_id.principal_id === principal) });
    }
    if (/^\/_coland\/admin\/realms\/[^/]+\/links$/.test(path) && method === 'GET') {
      const realmId = path.split('/')[4];
      const target = realmId === layoutIds.realm ? layoutIds.secondRealm : layoutIds.realm;
      const direction = url.searchParams.get('direction') || 'both';
      const links = [
        { realm_id: realmId, target_realm_id: target, link_kind: 'governed_by', status: 'active', label: 'Governance relationship', created_at: created, updated_at: recent },
        { realm_id: target, target_realm_id: realmId, link_kind: 'discoverable_from', status: 'active', label: 'Member directory discovery', created_at: created, updated_at: recent },
        { realm_id: realmId, target_realm_id: target, link_kind: 'mirror_of', status: 'tombstoned', label: 'Retired disaster recovery mirror', created_at: created, updated_at: recent },
      ];
      return reply({ realm_id: realmId, direction, links: links.filter(row => direction === 'both' || (direction === 'inbound' ? row.target_realm_id === realmId : row.realm_id === realmId)) });
    }
    if (/^\/_coland\/admin\/realms\/[^/]+\/media-service$/.test(path) && method === 'GET') {
      return reply({ realm_id: path.split('/')[4], service_id: 'ak:did_core:web:media.example', foci: [
        { focus_id: 'engineering-apac', focus_kind: 'livekit', region: 'apac-east', token_endpoint: 'https://media.example/apac/token', connect_url: 'wss://media.example/apac', capabilities: ['audio', 'video', 'screen'], health_endpoint: 'https://media.example/apac/health', cascade_group: 'engineering' },
        { focus_id: 'engineering-eu', focus_kind: 'arkret_native', region: 'eu-central', token_endpoint: 'https://media.example/eu/token', connect_url: 'wss://media.example/eu', capabilities: ['audio', 'video'], health_endpoint: 'https://media.example/eu/health', cascade_group: 'engineering' },
      ] });
    }
    if (/^\/_coland\/admin\/realms\/[^/]+\/organizations$/.test(path) && method === 'GET') {
      return reply({ realm_id: path.split('/')[4], declared_organization_hint_ids: [partnerOrganizationId], relationships: [
        { statement_id: 'organization-statement-1', organization_id: organizationId, relationship: 'owner', status: 'active', control_scopes: ['official_badge', 'directory_listing'], issued_at: created, expires_at: expires, realm_commit_ref: layoutIds.realm.replace('ak:realm:', 'ak:realm_commit:'), issuer_role: 'organization', lifecycle_phase: 'verified_active', updated_at: recent },
        { statement_id: 'organization-statement-2', organization_id: partnerOrganizationId, relationship: 'sponsor', status: 'revoked', control_scopes: ['directory_listing'], issued_at: created, expires_at: recent, revokes_statement_id: 'organization-statement-original', issuer_role: 'account_authority', delegation_ref: 'organization-delegation-2', lifecycle_phase: 'revoked_or_expired', updated_at: recent },
      ] });
    }
    if (path.startsWith('/_coauth/admin/organizations/') && method === 'GET') {
      const organization = path.slice('/_coauth/admin/organizations/'.length);
      if (organization === organizationId || organization === partnerOrganizationId) return reply(organizationControl(organization, organization === partnerOrganizationId));
    }

    if (path === '/_coland/admin/actors' && method === 'GET') return reply({ actors, total: actors.length, next_cursor: null, has_more: false, filters: {} });
    if (path.startsWith('/_coland/admin/actors/') && method === 'GET') return reply(actors.find(row => row.id === path.slice('/_coland/admin/actors/'.length)) || actors[0]);
    if (path === '/_coland/admin/realms' && method === 'GET') return reply(collection(realms));
    if (/^\/_coland\/admin\/realms\/[^/]+$/.test(path) && method === 'GET') return reply(realms.find(row => row.id === path.split('/').at(-1)) || realms[0]);
    if (path === '/_coland/admin/spaces' && method === 'GET') return reply(collection(spaces));
    // Federation detail deliberately scans this same listing (limit=100);
    // coland exposes no per-operation detail endpoint.
    if (path === '/_coland/admin/federation' && method === 'GET') return reply(collection(federation));
    if (path === '/_coland/admin/media' && method === 'GET') return reply(collection(media));
    if (path === '/_coland/admin/media/statistics') return reply({ total_blobs: 46, total_size: 12943360, encrypted_count: 34, quarantined_count: 0, by_media_type: { 'image/png': { count: 34, total_size: 10485760 }, 'application/pdf': { count: 12, total_size: 2457600 } }, by_realm: { [layoutIds.realm]: { count: 34, total_size: 10485760 }, [layoutIds.secondRealm]: { count: 12, total_size: 2457600 } }, by_actor: mediaActors });
    if (path === '/_coland/admin/media/by-actor') return reply(collection(mediaActors));
    if (path === '/_coland/admin/devices') return reply({ devices: devices.map(row => ({ id: row.id, actor_id: layoutIds.actor, display_name: row.display_name, verification_state: row.mfa_state === 'verified' ? 'verified' : 'unverified', payload: {}, created_at: row.registered_at, updated_at: recent, revoked_at: row.revoked_at })), total: 2, next_cursor: null, has_more: false, filters: {} });
    if (path === '/_coland/admin/audit') return reply({ entries: [
      { id: 'station-audit-1', request_id: 'request-2048', action: 'admin.devices.revoke', actor_id: layoutIds.actor, device_id: layoutIds.device, realm_id: layoutIds.realm, operation_id: null, outcome: 'accepted', payload: { reason: 'Security rotation', executed_by: layoutIds.actor }, created_at: recent },
      { id: 'station-audit-2', request_id: 'request-2047', action: 'admin.realms.query', actor_id: layoutIds.secondActor, device_id: null, realm_id: layoutIds.secondRealm, operation_id: null, outcome: 'accepted', payload: { scope: 'deployment-admin' }, created_at: created },
    ], total: 2, next_cursor: null, has_more: false, filters: {} });
    if (path === '/_coland/admin/handles') return reply(collection(handles));
    if (/^\/_coland\/admin\/handles\/[^/]+$/.test(path)) return reply(handles.find(row => row.id === path.split('/').at(-1)) || handles[0]);
    if (path.endsWith('/audit') && path.startsWith('/_coland/admin/handles/')) return reply(collection([{ id: 'handle-audit-1', action: 'assign', actor_id: layoutIds.actor, timestamp: created, reason: 'Organization onboarding', previous_subject_id: null, new_subject_id: layoutIds.actor }]));
    if (path === '/_coland/admin/invite-tokens') return reply(collection([
      { kind: 'invite_token', id: 'invite-1234', invite_id: 'invite-1234', realm_id: layoutIds.realm, inviter_id: layoutIds.actor, created_by: layoutIds.actor, invitee_id: null, introduction_evidence_digest: null, status: 'active', uses_allowed: 10, uses_completed: 3, uses_pending: 2, expires_at: expires, created_at: created },
      { kind: 'invite_token', id: 'invite-5678', invite_id: 'invite-5678', realm_id: layoutIds.secondRealm, inviter_id: layoutIds.actor, created_by: layoutIds.actor, invitee_id: layoutIds.secondActor, introduction_evidence_digest: null, status: 'expired', uses_allowed: 1, uses_completed: 1, uses_pending: 0, expires_at: created, created_at: created },
    ]));
    if (path === '/_coland/self/policies') return reply({ policies: [
      { policy_id: 'policy-1234', owner: layoutIds.actor, scope: 'realm', subject_ref: layoutIds.realm, policy_kind: 'ak.message.create', payload: { effect: 'allow', actions: ['ak.message.create'], resource: { realm_id: layoutIds.realm }, obligations: [] }, active: true, updated_at: recent },
      { policy_id: 'policy-5678', owner: layoutIds.actor, scope: 'realm', subject_ref: layoutIds.secondRealm, policy_kind: 'ak.media.upload', payload: { effect: 'deny', actions: ['ak.media.upload'], resource: { realm_id: layoutIds.secondRealm }, obligations: [] }, active: false, updated_at: created },
    ], next_cursor: null });
    if (path === '/_coland/admin/service-routes') return reply({ routes: [
      { service_id: layoutIds.station, service_kind: 'station', known: true, method_history_head: 'route-history-1', version_id: 'route-v1', cache_expires_at: expires, quarantined: false },
      { service_id: 'ak:did_core:web:partner.example', service_kind: 'station', known: true, method_history_head: 'route-history-2', version_id: 'route-v1', cache_expires_at: recent, quarantined: true },
    ], next_cursor: null, has_more: false });
    if (path.startsWith('/_coland/admin/service-routes/')) return reply({ service_id: layoutIds.station, service_kind: 'station', known: true, method_state: { did: 'did:web:station.example', method_history_head: 'route-history-1', version_id: 'route-v1', verified_at: recent }, current_route: { did: 'did:web:station.example', method_history_head: 'route-history-1', version_id: 'route-v1', base_url: 'https://station.example', verified_at: recent }, cache: { verified_at: recent, cache_expires_at: expires, routable_at_observed_at: true }, quarantine: [], quarantine_truncated: false, observed_at: recent, authority: 'verified_method_state' });
    if (path === '/health') return reply({ hardening: { development_mode: false, tls_enabled: true, pq_hybrid_tls_required_group: 'X25519MLKEM768', pq_hybrid_tls_probe_artifact: 'layout-fixture', pq_hybrid_tls_probe_verified: true, csp_header_configured: true, cors_strict: true, secret_manager_in_use: true, log_redaction_enabled: true, admin_auth_mode: 'oidc', rate_limit_enabled: true, provider_credential_rotation: 'automated', checklist_score: 10, checklist_max: 10, warnings: [] } });
    if (path.startsWith('/_')) {
      missing.push(`${method} ${path}`);
      errors.push(`Layout fixture missing: ${method} ${path}`);
      return route.fulfill({ status: 503, json: { type: 'about:blank', title: 'Layout fixture missing', status: 503, code: 'service_unavailable', detail: `No contract fixture for ${method} ${path}` } });
    }
    return route.continue();
  });
  return errors;
}

use glory::prelude::*;

use crate::api::auth;
use crate::pages;
use crate::utils::i18n::t;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub page: Page,
    pub params: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Login,
    Callback,
    NotFound,
    Dashboard,
    ActorList,
    ActorShow,
    RealmList,
    RealmShow,
    RealmAuthority,
    SpaceList,
    SpaceShow,
    MediaList,
    FederationList,
    FederationShow,
    DeviceList,
    CapabilityList,
    HandleList,
    HandleShow,
    RealmLinks,
    RealmMediaService,
    RealmOrganization,
    KeyBackupList,
    AuditLog,
    InviteTokenList,
    PolicyList,
    ServerStatus,
    ServiceRouteList,
    ServiceRouteShow,
    HardeningDashboard,
    CoauthAuditLog,
    CoauthAccountList,
    CoauthAccountShow,
    CoauthAccountDevices,
    CoauthCapabilities,
    CoauthOAuth2Sessions,
    CoauthPersonalSessions,
    CoauthUpstreamProviders,
    CoauthUpstreamLinks,
    CoauthRegistrationTokens,
    CoauthNotificationChannels,
    CoauthNotificationTemplates,
    CoauthConnectorHealth,
}

pub fn resolve(path: &str) -> Route {
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let raw: Vec<_> = path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let decoded: Option<Vec<String>> = raw
        .iter()
        .map(|part| urlencoding::decode(part).ok().map(|v| v.into_owned()))
        .collect();
    let Some(parts) = decoded else {
        return Route {
            page: Page::NotFound,
            params: vec![],
        };
    };
    let parts: Vec<_> = parts.iter().map(String::as_str).collect();
    let (page, params) = match parts.as_slice() {
        ["login"] => (Page::Login, vec![]),
        ["oauth", "callback"] => (Page::Callback, vec![]),
        [] => (Page::Dashboard, vec![]),
        ["actors"] => (Page::ActorList, vec![]),
        ["actors", p1] => (Page::ActorShow, vec![p1.to_string()]),
        ["realms"] => (Page::RealmList, vec![]),
        ["realms", p1] => (Page::RealmShow, vec![p1.to_string()]),
        ["realms", p1, "authority"] => (Page::RealmAuthority, vec![p1.to_string()]),
        ["spaces"] => (Page::SpaceList, vec![]),
        ["spaces", p1] => (Page::SpaceShow, vec![p1.to_string()]),
        ["media"] => (Page::MediaList, vec![]),
        ["federation"] => (Page::FederationList, vec![]),
        ["federation", p1] => (Page::FederationShow, vec![p1.to_string()]),
        ["devices"] => (Page::DeviceList, vec![]),
        ["capabilities"] => (Page::CapabilityList, vec![]),
        ["handles"] => (Page::HandleList, vec![]),
        ["handles", p1] => (Page::HandleShow, vec![p1.to_string()]),
        ["realms", p1, "links"] => (Page::RealmLinks, vec![p1.to_string()]),
        ["realms", p1, "media-service"] => (Page::RealmMediaService, vec![p1.to_string()]),
        ["realms", p1, "organization"] => (Page::RealmOrganization, vec![p1.to_string()]),
        ["key-backup"] => (Page::KeyBackupList, vec![]),
        ["audit"] => (Page::AuditLog, vec![]),
        ["invite-tokens"] => (Page::InviteTokenList, vec![]),
        ["policy"] => (Page::PolicyList, vec![]),
        ["server-status"] => (Page::ServerStatus, vec![]),
        ["service-routes"] => (Page::ServiceRouteList, vec![]),
        ["service-routes", p1, p2] => {
            (Page::ServiceRouteShow, vec![p1.to_string(), p2.to_string()])
        }
        ["hardening"] => (Page::HardeningDashboard, vec![]),
        ["coauth", "audit-log"] => (Page::CoauthAuditLog, vec![]),
        ["coauth", "accounts"] => (Page::CoauthAccountList, vec![]),
        ["coauth", "accounts", p2] => (Page::CoauthAccountShow, vec![p2.to_string()]),
        ["coauth", "accounts", p2, "devices"] => (Page::CoauthAccountDevices, vec![p2.to_string()]),
        ["coauth", "capabilities"] => (Page::CoauthCapabilities, vec![]),
        ["coauth", "oauth2-sessions"] => (Page::CoauthOAuth2Sessions, vec![]),
        ["coauth", "personal-sessions"] => (Page::CoauthPersonalSessions, vec![]),
        ["coauth", "upstream-providers"] => (Page::CoauthUpstreamProviders, vec![]),
        ["coauth", "upstream-links"] => (Page::CoauthUpstreamLinks, vec![]),
        ["coauth", "registration-tokens"] => (Page::CoauthRegistrationTokens, vec![]),
        ["coauth", "notification-channels"] => (Page::CoauthNotificationChannels, vec![]),
        ["coauth", "notification-templates"] => (Page::CoauthNotificationTemplates, vec![]),
        ["coauth", "connector-health"] => (Page::CoauthConnectorHealth, vec![]),
        _ => (Page::NotFound, vec![]),
    };
    Route { page, params }
}

pub fn current_path() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_else(|| "/".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "/".into()
    }
}
pub fn navigate(path: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        let _ = window.location().assign(path);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = path;
}

pub fn app(_scope: &Scope, theme: sapid::Theme) -> impl Widget + use<> {
    let route = resolve(&current_path());
    widget(move |scope| match route.page {
        Page::Login => pages::login::page(scope, theme.clone()).show_in(scope),
        Page::Callback => pages::oauth_callback::page(scope, theme.clone()).show_in(scope),
        _ if !auth::is_authenticated() => {
            navigate("/login");
            sapid::anchor()
                .theme(theme.clone())
                .href("/login")
                .label(t("auth.sign_in"))
                .show_in(scope)
        }
        _ => authenticated(scope, theme.clone(), route.clone()).show_in(scope),
    })
}

fn authenticated(scope: &Scope, theme: sapid::Theme, route: Route) -> impl Widget + use<> {
    let verdict = scope.cage(None::<bool>);
    let message = scope.cage(t("common.loading"));
    let busy = scope.cage(false);
    probe(scope, verdict, message, busy);
    let allowed = scope.bond(move || verdict.get() == Some(true));
    let rejected = scope.bond(move || verdict.get() == Some(false));
    let no_theme = theme.clone();
    let wait_theme = theme.clone();
    let weak = scope.downgrade();
    when(allowed, move || shell(theme.clone(), route.clone())).otherwise(move || {
        let denied_theme = no_theme.clone();
        let pending_theme = wait_theme.clone();
        let weak = weak.clone();
        when(rejected, move || {
            widget({
                let theme = denied_theme.clone();
                move |scope| pages::not_authorized::page(scope, theme.clone()).show_in(scope)
            })
        })
        .otherwise(move || {
            col().class("session-check").fill((
                sapid::text().theme(pending_theme.clone()).content(message),
                sapid::button()
                    .theme(pending_theme.clone())
                    .label(t("common.retry"))
                    .enabled(scope_enabled(busy))
                    .on_press({
                        let weak = weak.clone();
                        move || {
                            if let Some(scope) = weak.upgrade() {
                                probe(&scope, verdict, message, busy);
                            }
                        }
                    }),
            ))
        })
    })
}
fn scope_enabled(busy: Cage<bool>) -> glory::Lotus<bool> {
    busy.map(|busy| !*busy).into()
}
fn probe(scope: &Scope, verdict: Cage<Option<bool>>, message: Cage<String>, busy: Cage<bool>) {
    if busy.get_untracked() {
        return;
    }
    busy.revise(|v| *v = true);
    let result = scope.owner().spawn(async move {
        match auth::verify_admin().await {
            Ok(allowed) => verdict.revise(|v| *v = Some(allowed)),
            Err(err) if err.status == 401 || err.status == 403 => {
                verdict.revise(|v| *v = Some(false))
            }
            Err(err) => message.revise(|v| *v = format!("Admin check failed: {}", err.message)),
        }
        busy.revise(|v| *v = false);
    });
    if let Err(err) = result {
        message.revise(|v| *v = err.to_string());
        busy.revise(|v| *v = false);
    }
}

fn shell(theme: sapid::Theme, route: Route) -> impl Widget {
    let nav_theme = theme.clone();
    let header_theme = theme.clone();
    let main_theme = theme.clone();
    sapid::app_shell("codmin")
        .theme(theme.clone())
        .class("codmin-shell")
        .sidebar(
            sapid::sidebar("primary")
                .name(t("nav.dashboard"))
                .content(move |_| navigation(nav_theme.clone(), route.page)),
        )
        .header(move |context| {
            widget({
                let theme = header_theme.clone();
                move |scope| {
                    let sidebar = context.sidebar.clone();
                    let show_help = scope.cage(false);
                    crate::browser::session_sync(scope)?;
                    crate::browser::keyboard_shortcuts(scope, show_help)?;
                    let describe = crate::components::ui::resource(scope, || async { Ok(crate::api::server::get_server_describe().await.ok()) });
                    let dev_mode = describe.data.map(|value| value.as_ref().is_some_and(|v| v.as_ref().is_ok_and(|describe| describe.as_ref().is_some_and(|d| d.development_mode))));
                    let error = scope.cage(String::new());
                    let busy = scope.cage(false);
                    let weak = scope.downgrade();
                    let user = crate::utils::net::session::current_user();
                    let controls = row()
                        .class("codmin-header")
                        .fill((
                            sapid::button().theme(theme.clone()).label(t("nav.menu")).on_press(
                                move || {
                                    if let Some(handle) = &sidebar {
                                        let _ = handle.toggle();
                                    }
                                },
                            ),
                            sapid::title(1).theme(theme.clone()).content("Arkret Admin"),
                            sapid::text()
                                .theme(theme.clone())
                                .content(user.display_name.or(user.id).unwrap_or_default()),
                            sapid::button()
                                .theme(theme.clone())
                                .label(t("auth.sign_out"))
                                .loading(busy)
                                .on_press(move || {
                                    if busy.get_untracked() {
                                        return;
                                    }
                                    busy.revise(|v| *v = true);
                                    if let Some(scope) = weak.upgrade() {
                                        let _ = scope.owner().spawn(async move {
                                            let report = auth::logout().await;
                                            if report.local_cookie_cleared() {
                                                let path = report
                                                    .login_warning_code()
                                                    .map(|s| {
                                                        format!(
                                                            "/login?logout_warning={}",
                                                            urlencoding::encode(&s)
                                                        )
                                                    })
                                                    .unwrap_or_else(|| "/login".into());
                                                navigate(&path);
                                            } else {
                                                error.revise(|v| *v = report.retry_message());
                                            }
                                            busy.revise(|v| *v = false);
                                        });
                                    }
                                }),
                            sapid::button()
                                .theme(theme.clone())
                                .label("EN / 中文")
                                .on_press(|| {
                                    use crate::utils::i18n::*;
                                    set_language(if current_language() == Language::En {
                                        Language::ZhCn
                                    } else {
                                        Language::En
                                    });
                                    navigate(&current_path());
                                }),
                            sapid::button()
                                .theme(theme.clone())
                                .label(t("nav.theme"))
                                .on_press(|| {
                                    let next = if crate::components::theme::get_resolved_theme()
                                        == "dark"
                                    {
                                        "light"
                                    } else {
                                        "dark"
                                    };
                                    crate::components::theme::set_theme(next);
                                    navigate(&current_path());
                                }),
                            sapid::button().theme(theme.clone()).label(t("keyboard_shortcuts.title")).on_press(move || show_help.revise(|v| *v = !*v)),
                            sapid::text().theme(theme.clone()).content(error),
                        ));
                    let help_theme=theme.clone();
                    let warning_theme=theme.clone();
                    col().fill((
                        when(dev_mode,move || sapid::alert("development-mode").theme(warning_theme.clone()).severity(sapid::AlertSeverity::Danger).content(t("server_status.dev_banner"))),
                        controls,
                        when(show_help,move || sapid::card().theme(help_theme.clone()).title(t("keyboard_shortcuts.title")).fill((
                            text().content("g d: Dashboard · g a: Actors · g s: Spaces · g f: Federation · g v: Devices · g c: Accounts · g l: Audit · g p: Policy · g t: Server Status · g e: Capabilities"),
                            sapid::button().theme(help_theme.clone()).label(t("common.close")).on_press(move || show_help.revise(|v|*v=false)),
                        ))),
                    )).show_in(scope)
                }
            })
        })
        .main(move |_| content(main_theme.clone(), route.clone()))
}
fn navigation(theme: sapid::Theme, current: Page) -> impl Widget {
    widget(move |scope| {
        let theme = theme.clone();
        let nav = col().class("codmin-navigation").fill(widget(move |scope| {
            sapid::anchor()
                .theme(theme.clone())
                .href("/")
                .label(t("nav.dashboard"))
                .current((current == Page::Dashboard).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/actors")
                .label(t("nav.actors"))
                .current((current == Page::ActorList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/realms")
                .label(t("nav.realms"))
                .current((current == Page::RealmList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/spaces")
                .label(t("nav.spaces"))
                .current((current == Page::SpaceList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/media")
                .label(t("nav.media"))
                .current((current == Page::MediaList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/federation")
                .label(t("nav.federation"))
                .current((current == Page::FederationList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/devices")
                .label(t("nav.devices"))
                .current((current == Page::DeviceList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/capabilities")
                .label(t("nav.capabilities"))
                .current((current == Page::CapabilityList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/handles")
                .label(t("nav.handles"))
                .current((current == Page::HandleList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/key-backup")
                .label("Key backup")
                .current((current == Page::KeyBackupList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/audit")
                .label(t("nav.audit"))
                .current((current == Page::AuditLog).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/invite-tokens")
                .label(t("nav.invite_tokens"))
                .current((current == Page::InviteTokenList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/policy")
                .label(t("nav.policy"))
                .current((current == Page::PolicyList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/server-status")
                .label(t("nav.server_status"))
                .current((current == Page::ServerStatus).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/service-routes")
                .label(t("nav.service_routes"))
                .current((current == Page::ServiceRouteList).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/hardening")
                .label("Hardening")
                .current((current == Page::HardeningDashboard).then_some(Current::Page))
                .show_in(scope)?;
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/audit-log")
                    .label(t("nav.audit_log"))
                    .current((current == Page::CoauthAuditLog).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/accounts")
                    .label(t("nav.coauth_accounts"))
                    .current((current == Page::CoauthAccountList).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/capabilities")
                    .label(t("nav.coauth_capabilities"))
                    .current((current == Page::CoauthCapabilities).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/oauth2-sessions")
                    .label(t("nav.oauth2_sessions"))
                    .current((current == Page::CoauthOAuth2Sessions).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/personal-sessions")
                    .label(t("nav.personal_tokens"))
                    .current((current == Page::CoauthPersonalSessions).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/upstream-providers")
                    .label(t("nav.upstream_providers"))
                    .current((current == Page::CoauthUpstreamProviders).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/upstream-links")
                    .label(t("nav.upstream_links"))
                    .current((current == Page::CoauthUpstreamLinks).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/registration-tokens")
                    .label(t("nav.registration_tokens"))
                    .current((current == Page::CoauthRegistrationTokens).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/notification-channels")
                    .label(t("nav.notification_channels"))
                    .current((current == Page::CoauthNotificationChannels).then_some(Current::Page))
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/notification-templates")
                    .label(t("nav.notification_templates"))
                    .current(
                        (current == Page::CoauthNotificationTemplates).then_some(Current::Page),
                    )
                    .show_in(scope)?;
            }
            if crate::utils::net::session::has_coauth() {
                sapid::anchor()
                    .theme(theme.clone())
                    .href("/coauth/connector-health")
                    .label(t("nav.connector_health"))
                    .current((current == Page::CoauthConnectorHealth).then_some(Current::Page))
                    .show_in(scope)?;
            }
            sapid::anchor()
                .theme(theme.clone())
                .href("/realms/_/links")
                .label(t("nav.realm_links"))
                .current((current == Page::RealmLinks).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/realms/_/media-service")
                .label(t("media_service.title"))
                .current((current == Page::RealmMediaService).then_some(Current::Page))
                .show_in(scope)?;
            sapid::anchor()
                .theme(theme.clone())
                .href("/realms/_/organization")
                .label(t("realm_organization.title"))
                .current((current == Page::RealmOrganization).then_some(Current::Page))
                .show_in(scope)?;
            Ok(())
        }));
        nav.show_in(scope)
    })
}
fn content(theme: sapid::Theme, route: Route) -> impl Widget {
    widget(move |scope| match route.page {
        Page::Dashboard => pages::dashboard::page(scope, theme.clone()).show_in(scope),
        Page::ActorList => pages::actors::list::page(scope, theme.clone()).show_in(scope),
        Page::ActorShow => {
            pages::actors::show::page(scope, theme.clone(), route.params[0].clone()).show_in(scope)
        }
        Page::RealmList => pages::realms::list::page(scope, theme.clone()).show_in(scope),
        Page::RealmShow => {
            pages::realms::show::page(scope, theme.clone(), route.params[0].clone()).show_in(scope)
        }
        Page::RealmAuthority => {
            pages::realms::authority::page(scope, theme.clone(), route.params[0].clone())
                .show_in(scope)
        }
        Page::SpaceList => pages::spaces::list::page(scope, theme.clone()).show_in(scope),
        Page::SpaceShow => {
            pages::spaces::show::page(scope, theme.clone(), route.params[0].clone()).show_in(scope)
        }
        Page::MediaList => pages::media::page(scope, theme.clone()).show_in(scope),
        Page::FederationList => pages::federation::list::page(scope, theme.clone()).show_in(scope),
        Page::FederationShow => {
            pages::federation::show::page(scope, theme.clone(), route.params[0].clone())
                .show_in(scope)
        }
        Page::DeviceList => pages::devices::page(scope, theme.clone()).show_in(scope),
        Page::CapabilityList => pages::capabilities::page(scope, theme.clone()).show_in(scope),
        Page::HandleList => pages::handles::page(scope, theme.clone()).show_in(scope),
        Page::HandleShow => {
            pages::handles::show_page(scope, theme.clone(), route.params[0].clone()).show_in(scope)
        }
        Page::RealmLinks => {
            pages::realms::links::page(scope, theme.clone(), route.params[0].clone()).show_in(scope)
        }
        Page::RealmMediaService => {
            pages::realms::media_service::page(scope, theme.clone(), route.params[0].clone())
                .show_in(scope)
        }
        Page::RealmOrganization => {
            pages::realms::organization::page(scope, theme.clone(), route.params[0].clone())
                .show_in(scope)
        }
        Page::KeyBackupList => pages::key_backup::page(scope, theme.clone()).show_in(scope),
        Page::AuditLog => pages::audit::page(scope, theme.clone()).show_in(scope),
        Page::InviteTokenList => pages::invite_tokens::page(scope, theme.clone()).show_in(scope),
        Page::PolicyList => pages::policy::page(scope, theme.clone()).show_in(scope),
        Page::ServerStatus => pages::server_status::page(scope, theme.clone()).show_in(scope),
        Page::ServiceRouteList => {
            pages::service_routes::list::page(scope, theme.clone()).show_in(scope)
        }
        Page::ServiceRouteShow => pages::service_routes::show::page(
            scope,
            theme.clone(),
            route.params[0].clone(),
            route.params[1].clone(),
        )
        .show_in(scope),
        Page::HardeningDashboard => pages::hardening::page(scope, theme.clone()).show_in(scope),
        Page::CoauthAuditLog => pages::coauth::audit_log::page(scope, theme.clone()).show_in(scope),
        Page::CoauthAccountList => {
            pages::coauth::accounts::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthAccountShow => {
            pages::coauth::account_detail::page(scope, theme.clone(), route.params[0].clone())
                .show_in(scope)
        }
        Page::CoauthAccountDevices => {
            pages::coauth::devices::page(scope, theme.clone(), route.params[0].clone())
                .show_in(scope)
        }
        Page::CoauthCapabilities => {
            pages::coauth::capabilities::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthOAuth2Sessions => {
            pages::coauth::oauth2_sessions::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthPersonalSessions => {
            pages::coauth::personal_sessions::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthUpstreamProviders => {
            pages::coauth::upstream_providers::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthUpstreamLinks => {
            pages::coauth::upstream_links::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthRegistrationTokens => {
            pages::coauth::registration_tokens::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthNotificationChannels => {
            pages::coauth::notification_channels::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthNotificationTemplates => {
            pages::coauth::notification_templates::page(scope, theme.clone()).show_in(scope)
        }
        Page::CoauthConnectorHealth => {
            pages::coauth::connector_health::page(scope, theme.clone()).show_in(scope)
        }
        _ => col()
            .fill((
                sapid::title(1)
                    .theme(theme.clone())
                    .content(t("not_found.title")),
                sapid::anchor()
                    .theme(theme.clone())
                    .label(t("not_found.go_dashboard"))
                    .href("/"),
            ))
            .show_in(scope),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_decode_parameters_and_reject_unknown_paths() {
        assert_eq!(
            resolve("/coauth/accounts/alice%40example/devices").page,
            Page::CoauthAccountDevices
        );
        assert_eq!(
            resolve("/coauth/accounts/alice%40example/devices").params,
            ["alice@example"]
        );
        assert_eq!(
            resolve("/realms/realm-1/authority").page,
            Page::RealmAuthority
        );
        assert_eq!(
            resolve("/oauth/callback?code=abc&state=def").page,
            Page::Callback
        );
        assert_eq!(resolve("/unknown").page, Page::NotFound);
    }
}

use dioxus::prelude::*;

use crate::api::auth;
use crate::components::layout::AppLayout;
use crate::pages;
use crate::utils::i18n::t;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login?:logout_warning")]
    LoginPage { logout_warning: Option<String> },

    #[route("/oauth/callback?:code&:state&:error&:error_description")]
    OAuthCallback {
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    },

    #[layout(AuthenticatedLayout)]
        #[route("/")]
        Dashboard {},

        #[route("/actors")]
        ActorList {},
        #[route("/actors/:actor_id")]
        ActorShow { actor_id: String },

        #[route("/realms")]
        RealmList {},
        #[route("/realms/:realm_id")]
        RealmShow { realm_id: String },
        #[route("/realms/:realm_id/authority")]
        RealmAuthority { realm_id: String },
        #[route("/spaces")]
        SpaceList {},
        #[route("/spaces/:space_id")]
        SpaceShow { space_id: String },

        #[route("/media")]
        MediaList {},

        #[route("/federation")]
        FederationList {},
        #[route("/federation/:operation_id")]
        FederationShow { operation_id: String },

        #[route("/devices")]
        DeviceList {},

        #[route("/capabilities")]
        CapabilityList {},

        #[route("/handles")]
        HandleList {},
        #[route("/handles/:handle_id")]
        HandleShow { handle_id: String },

        #[route("/realms/:realm_id/links")]
        RealmLinks { realm_id: String },

        #[route("/realms/:realm_id/media-service")]
        RealmMediaService { realm_id: String },

        #[route("/realms/:realm_id/organization")]
        RealmOrganization { realm_id: String },

        #[route("/key-backup")]
        KeyBackupList {},

        #[route("/audit")]
        AuditLog {},

        #[route("/invite-tokens")]
        InviteTokenList {},

        #[route("/policy")]
        PolicyList {},

        #[route("/server-status")]
        ServerStatus {},

        #[route("/service-routes")]
        ServiceRouteList {},
        #[route("/service-routes/:service_id/:service_kind")]
        ServiceRouteShow { service_id: String, service_kind: String },

        #[route("/hardening")]
        HardeningDashboard {},

        #[route("/coauth/audit-log")]
        CoauthAuditLog {},
        #[route("/coauth/accounts")]
        CoauthAccountList {},
        #[route("/coauth/accounts/:account_id")]
        CoauthAccountShow { account_id: String },
        #[route("/coauth/accounts/:account_id/devices")]
        CoauthAccountDevices { account_id: String },
        #[route("/coauth/capabilities")]
        CoauthCapabilities {},
        #[route("/coauth/oauth2-sessions")]
        CoauthOAuth2Sessions {},
        #[route("/coauth/personal-sessions")]
        CoauthPersonalSessions {},
        #[route("/coauth/upstream-providers")]
        CoauthUpstreamProviders {},
        #[route("/coauth/upstream-links")]
        CoauthUpstreamLinks {},
        #[route("/coauth/registration-tokens")]
        CoauthRegistrationTokens {},
        #[route("/coauth/notification-channels")]
        CoauthNotificationChannels {},
        #[route("/coauth/notification-templates")]
        CoauthNotificationTemplates {},
        #[route("/coauth/connector-health")]
        CoauthConnectorHealth {},
    #[end_layout]

    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[component]
pub fn AppRouter() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn AuthenticatedLayout() -> Element {
    let nav = use_navigator();

    if !auth::is_authenticated() {
        nav.replace(Route::LoginPage {
            logout_warning: None,
        });
        return rsx! {
            div { "Redirecting..." }
        };
    }

    // codmin treats coauth process-admin and Station admin reachability as one
    // operator role in the current deployment model. Backend calls still
    // enforce their own authorization; this layout only prevents obviously
    // non-admin users from entering either admin surface.
    let cached = auth::cached_is_admin();
    let mut admin_probe_error = use_signal(|| None::<String>);
    let mut admin_probe = use_resource(move || async move {
        match auth::verify_admin().await {
            Ok(flag) => {
                admin_probe_error.set(None);
                Some(flag)
            }
            Err(err) if err.status == 401 || err.status == 403 => Some(false),
            Err(err) if err.status == 0 || err.status >= 500 => {
                log::error!("admin probe failed: {}", err.message);
                if cached.is_none() {
                    admin_probe_error.set(Some(err.message));
                }
                cached
            }
            Err(_) => Some(false),
        }
    });

    let verdict = match admin_probe.read().as_ref() {
        Some(Some(v)) => Some(*v),
        Some(None) => cached,
        None => None,
    };

    match verdict {
        Some(true) => rsx! {
            AppLayout {
                Outlet::<Route> {}
            }
        },
        Some(false) => rsx! {
            pages::not_authorized::NotAuthorizedPage {}
        },
        None => rsx! {
            if let Some(message) = admin_probe_error.read().clone() {
                div { class: "flex min-h-screen items-center justify-center p-6",
                    div { class: "max-w-md rounded-md border p-4 text-center space-y-3",
                        h1 { class: "text-lg font-semibold", "Admin check failed" }
                        p { class: "text-sm text-muted-foreground", "{message}" }
                        button {
                            class: "inline-flex h-10 items-center rounded-md border px-4 text-sm font-medium transition-colors hover:bg-accent",
                            onclick: move |_| {
                                admin_probe_error.set(None);
                                admin_probe.restart();
                            },
                            "Retry"
                        }
                    }
                }
            } else {
                div { class: "flex min-h-screen items-center justify-center",
                    crate::components::ui::loading::Spinner { class: String::new() }
                }
            }
        },
    }
}

#[component]
fn LoginPage(logout_warning: Option<String>) -> Element {
    rsx! {
        pages::login::LoginPage { logout_warning }
    }
}

#[component]
fn OAuthCallback(
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
) -> Element {
    rsx! {
        pages::oauth_callback::OAuthCallback {
            code: code,
            state: state,
            error: error,
            error_description: error_description,
        }
    }
}

#[component]
fn Dashboard() -> Element {
    rsx! { pages::dashboard::Dashboard {} }
}

#[component]
fn ActorList() -> Element {
    rsx! { pages::actors::list::ActorList {} }
}

#[component]
fn ActorShow(actor_id: String) -> Element {
    rsx! { pages::actors::show::ActorShow { actor_id } }
}

#[component]
fn RealmList() -> Element {
    rsx! { pages::realms::list::RealmList {} }
}

#[component]
fn RealmShow(realm_id: String) -> Element {
    rsx! { pages::realms::show::RealmShow { realm_id } }
}

#[component]
fn RealmAuthority(realm_id: String) -> Element {
    rsx! { pages::realms::authority::RealmAuthorityPage { realm_id } }
}

#[component]
fn SpaceList() -> Element {
    rsx! { pages::spaces::list::SpaceList {} }
}

#[component]
fn SpaceShow(space_id: String) -> Element {
    rsx! { pages::spaces::show::SpaceShow { space_id } }
}

#[component]
fn MediaList() -> Element {
    rsx! { pages::media::MediaList {} }
}

#[component]
fn FederationList() -> Element {
    rsx! { pages::federation::list::FederationList {} }
}

#[component]
fn FederationShow(operation_id: String) -> Element {
    rsx! { pages::federation::show::FederationShow { operation_id } }
}

#[component]
fn ServiceRouteList() -> Element {
    rsx! { pages::service_routes::list::ServiceRouteList {} }
}

#[component]
fn ServiceRouteShow(service_id: String, service_kind: String) -> Element {
    rsx! {
        pages::service_routes::show::ServiceRouteShow {
            service_id,
            service_kind,
        }
    }
}

#[component]
fn DeviceList() -> Element {
    rsx! { pages::devices::DeviceList {} }
}

#[component]
fn CapabilityList() -> Element {
    rsx! { pages::capabilities::CapabilityList {} }
}

#[component]
fn HandleList() -> Element {
    rsx! { pages::handles::HandleList {} }
}

#[component]
fn HandleShow(handle_id: String) -> Element {
    rsx! { pages::handles::HandleShow { handle_id } }
}

#[component]
#[component]
fn RealmLinks(realm_id: String) -> Element {
    rsx! { pages::realms::links::LinksPage { realm_id } }
}

#[component]
fn RealmMediaService(realm_id: String) -> Element {
    rsx! { pages::realms::media_service::MediaServicePage { realm_id } }
}

#[component]
fn RealmOrganization(realm_id: String) -> Element {
    rsx! { pages::realms::organization::OrganizationPage { realm_id } }
}

#[component]
fn KeyBackupList() -> Element {
    rsx! { pages::key_backup::KeyBackupList {} }
}

#[component]
fn AuditLog() -> Element {
    rsx! { pages::audit::AuditLog {} }
}

#[component]
fn InviteTokenList() -> Element {
    rsx! { pages::invite_tokens::InviteTokenList {} }
}

#[component]
fn PolicyList() -> Element {
    rsx! { pages::policy::PolicyList {} }
}

#[component]
fn ServerStatus() -> Element {
    rsx! { pages::server_status::ServerStatus {} }
}

#[component]
fn HardeningDashboard() -> Element {
    rsx! { pages::hardening::HardeningDashboard {} }
}

#[component]
fn CoauthAuditLog() -> Element {
    rsx! { pages::coauth::audit_log::AuditLogPage {} }
}

#[component]
fn CoauthAccountList() -> Element {
    rsx! { pages::coauth::accounts::AccountsPage {} }
}

#[component]
fn CoauthAccountShow(account_id: String) -> Element {
    rsx! {
        div { key: "{account_id}",
            pages::coauth::account_detail::AccountDetailPage { account_id }
        }
    }
}

#[component]
fn CoauthAccountDevices(account_id: String) -> Element {
    rsx! {
        div { key: "{account_id}",
            pages::coauth::devices::AccountDevicesPage { account_id }
        }
    }
}

#[component]
fn CoauthCapabilities() -> Element {
    rsx! { pages::coauth::capabilities::AuthzCapabilitiesPage {} }
}

#[component]
fn CoauthOAuth2Sessions() -> Element {
    rsx! { pages::coauth::oauth2_sessions::OAuth2SessionsPage {} }
}

#[component]
fn CoauthPersonalSessions() -> Element {
    rsx! { pages::coauth::personal_sessions::PersonalSessionsPage {} }
}

#[component]
fn CoauthUpstreamProviders() -> Element {
    rsx! { pages::coauth::upstream_providers::UpstreamProvidersPage {} }
}

#[component]
fn CoauthUpstreamLinks() -> Element {
    rsx! { pages::coauth::upstream_links::UpstreamLinksPage {} }
}

#[component]
fn CoauthRegistrationTokens() -> Element {
    rsx! { pages::coauth::registration_tokens::RegistrationTokensPage {} }
}

#[component]
fn CoauthNotificationChannels() -> Element {
    rsx! { pages::coauth::notification_channels::NotificationChannelsPage {} }
}

#[component]
fn CoauthNotificationTemplates() -> Element {
    rsx! { pages::coauth::notification_templates::NotificationTemplatesPage {} }
}

#[component]
fn CoauthConnectorHealth() -> Element {
    rsx! { pages::coauth::connector_health::ConnectorHealthPage {} }
}

#[component]
fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "flex items-center justify-center min-h-screen",
            div { class: "text-center",
                h1 { class: "text-4xl font-bold mb-4", {t("not_found.title")} }
                p { class: "text-muted-foreground mb-4",
                    {format!("{}: /{}", t("not_found.message"), route.join("/"))}
                }
                Link { to: Route::Dashboard {}, class: "text-primary hover:underline",
                    {t("not_found.go_dashboard")}
                }
            }
        }
    }
}

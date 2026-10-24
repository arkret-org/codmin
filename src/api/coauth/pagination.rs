use serde::de::DeserializeOwned;

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::CursorPage;
use crate::utils::net::error::HttpError;

pub(crate) async fn get_jsonapi_cursor_page<T, U, F>(
    path: &str,
    after: Option<&str>,
    per_page: u64,
    map: F,
) -> Result<CursorPage<U>, HttpError>
where
    T: DeserializeOwned,
    F: Fn(coauth_admin_types::SingleResource<T>) -> U,
{
    get_jsonapi_cursor_page_with_query(path, after, per_page, &[], map).await
}

pub(crate) async fn get_jsonapi_cursor_page_with_query<T, U, F>(
    path: &str,
    after: Option<&str>,
    per_page: u64,
    query: &[(&str, &str)],
    map: F,
) -> Result<CursorPage<U>, HttpError>
where
    T: DeserializeOwned,
    F: Fn(coauth_admin_types::SingleResource<T>) -> U,
{
    let url = build_jsonapi_cursor_url(path, after, per_page, query)?;
    let resp: coauth_admin_types::PaginatedOutcome<T> = api_client(&url, "GET", NO_BODY).await?;
    Ok(flatten_jsonapi_page(resp, map))
}

fn build_jsonapi_cursor_url(
    path: &str,
    after: Option<&str>,
    per_page: u64,
    query: &[(&str, &str)],
) -> Result<String, HttpError> {
    let per_page = per_page.max(1).to_string();
    let mut parameters = vec![("page[first]", per_page.as_str()), ("count", "true")];
    if let Some(after) = after.filter(|value| !value.is_empty()) {
        parameters.push(("page[after]", after));
    }
    parameters.extend(query.iter().copied().filter(|(_, value)| !value.is_empty()));
    build_url(path, &parameters)
}

pub(crate) fn flatten_jsonapi_page<T, U, F>(
    resp: coauth_admin_types::PaginatedOutcome<T>,
    map: F,
) -> CursorPage<U>
where
    F: Fn(coauth_admin_types::SingleResource<T>) -> U,
{
    let next_cursor = resp.links.next.as_deref().and_then(extract_after_cursor);
    let data = resp
        .data
        .unwrap_or_default()
        .into_iter()
        .map(map)
        .collect::<Vec<_>>();
    let total = resp.meta.count.map(|count| count as u64);
    CursorPage {
        data,
        next_cursor,
        total,
    }
}

fn extract_after_cursor(link: &str) -> Option<String> {
    let query = link.split_once('?').map(|(_, query)| query)?;
    let query = query.split('#').next().unwrap_or(query);

    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        let key = urlencoding::decode(key).ok()?;
        if key != "page[after]" {
            return None;
        }
        let value = urlencoding::decode(value).ok()?.into_owned();
        (!value.is_empty()).then_some(value)
    })
}

pub(crate) fn map_single_resource<T, U, F>(resp: coauth_admin_types::SingleOutcome<T>, map: F) -> U
where
    F: Fn(coauth_admin_types::SingleResource<T>) -> U,
{
    map(resp.data)
}

#[cfg(test)]
mod tests {
    use super::{build_jsonapi_cursor_url, extract_after_cursor};

    #[test]
    fn builds_jsonapi_pagination_and_filter_query() {
        assert_eq!(
            build_jsonapi_cursor_url(
                "/_coauth/admin/accounts",
                Some("cursor/2"),
                25,
                &[("filter[search]", "alice")],
            )
            .expect("valid URL"),
            "/_coauth/admin/accounts?page%5Bfirst%5D=25&count=true&page%5Bafter%5D=cursor%2F2&filter%5Bsearch%5D=alice"
        );
    }

    #[test]
    fn extracts_literal_jsonapi_after_cursor() {
        assert_eq!(
            extract_after_cursor("/_coauth/admin/items?page[after]=item-2&page[first]=25"),
            Some("item-2".to_owned())
        );
    }

    #[test]
    fn extracts_encoded_jsonapi_after_cursor() {
        assert_eq!(
            extract_after_cursor(
                "/_coauth/admin/items?page%5Bafter%5D=item%2F2&page%5Bfirst%5D=25"
            ),
            Some("item/2".to_owned())
        );
    }

    #[test]
    fn rejects_missing_or_empty_jsonapi_after_cursor() {
        assert_eq!(
            extract_after_cursor("/_coauth/admin/items?page[first]=25"),
            None
        );
        assert_eq!(
            extract_after_cursor("/_coauth/admin/items?page[after]="),
            None
        );
        assert_eq!(extract_after_cursor("not-a-link"), None);
    }
}

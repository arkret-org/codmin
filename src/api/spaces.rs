//! HTTP client for the coland Space container admin surface.
//!
//! Endpoints:
//!
//! - `GET /_coland/admin/spaces`: list Spaces visible to the current admin scope. Cursor-paginated.
//!
//! Per-space hierarchy views are assembled client-side from the same snapshot;
//! coland does not expose a dedicated hierarchy endpoint.

use crate::api::client::{NO_BODY, api_client, build_url};
use crate::types::api::ListResponse;
use crate::types::spaces::{AdminSpaceView, SpaceHierarchy, SpaceHierarchyNode};
use crate::utils::net::error::HttpError;

#[derive(Debug, Clone, Default)]
pub struct AdminSpacePage {
    pub data: Vec<AdminSpaceView>,
    pub next_cursor: Option<String>,
    pub total_count: Option<u64>,
}

pub async fn list_spaces(
    cursor: Option<&str>,
    limit: u64,
    search: &str,
) -> Result<AdminSpacePage, HttpError> {
    let limit_str = limit.max(1).to_string();
    let mut parameters: Vec<(&str, &str)> = vec![("limit", limit_str.as_str())];
    if let Some(cursor) = cursor.filter(|c| !c.is_empty()) {
        parameters.push(("cursor", cursor));
    }
    let url = build_url("/_coland/admin/spaces", &parameters)?;
    let resp: ListResponse<AdminSpaceView> = api_client(&url, "GET", NO_BODY).await?;
    let needle = search.trim().to_ascii_lowercase();
    let data = if needle.is_empty() {
        resp.data
    } else {
        resp.data
            .into_iter()
            .filter(|row| {
                row.id.to_ascii_lowercase().contains(&needle)
                    || row.name.to_ascii_lowercase().contains(&needle)
            })
            .collect()
    };
    Ok(AdminSpacePage {
        data,
        next_cursor: resp.next_cursor,
        total_count: resp.total,
    })
}

pub async fn get_space(space_id: &str) -> Result<SpaceHierarchy, HttpError> {
    let rows = list_all_spaces().await?;
    let Some(center) = rows.iter().find(|row| row.id == space_id).cloned() else {
        return Err(HttpError::message("space not found in admin snapshot"));
    };
    let parent = center
        .parent_space_id
        .as_ref()
        .and_then(|parent_id| rows.iter().find(|row| row.id == *parent_id))
        .map(space_node);
    let children = rows
        .iter()
        .filter(|row| row.parent_space_id.as_deref() == Some(space_id))
        .map(space_node)
        .collect();
    Ok(SpaceHierarchy {
        space_id: center.id,
        name: center.name,
        parent,
        children,
    })
}

async fn list_all_spaces() -> Result<Vec<AdminSpaceView>, HttpError> {
    let mut cursor = None::<String>;
    let mut rows = Vec::new();
    loop {
        let page = list_spaces(cursor.as_deref(), 1000, "").await?;
        rows.extend(page.data);
        match page.next_cursor {
            Some(next) if !next.is_empty() && cursor.as_deref() != Some(next.as_str()) => {
                cursor = Some(next);
            }
            _ => break,
        }
    }
    Ok(rows)
}

fn space_node(row: &AdminSpaceView) -> SpaceHierarchyNode {
    SpaceHierarchyNode {
        space_id: row.id.clone(),
        name: row.name.clone(),
        member_count: row.member_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, parent: Option<&str>) -> AdminSpaceView {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": format!("Space {id}"),
            "realm_id": "ak:realm:ATumA-899wMAX4JSfO0b40Ppp5ik4VP0CG74lq-8UjYg",
            "kind": "list",
            "member_count": 3,
            "health": "active",
            "created_at": "2026-08-14T00:00:00.000Z",
            "parent_space_id": parent,
        }))
        .expect("shared space row should deserialize")
    }

    #[test]
    fn hierarchy_node_projects_the_shared_row() {
        let node = space_node(&row(
            "ak:space:AVX7ebly5NnDPvZ6X2AdCALS2TlVGPEDv2mXBOJl2YKj",
            None,
        ));
        assert_eq!(
            node.space_id,
            "ak:space:AVX7ebly5NnDPvZ6X2AdCALS2TlVGPEDv2mXBOJl2YKj"
        );
        assert_eq!(
            node.name,
            "Space ak:space:AVX7ebly5NnDPvZ6X2AdCALS2TlVGPEDv2mXBOJl2YKj"
        );
        assert_eq!(node.member_count, 3);
    }
}

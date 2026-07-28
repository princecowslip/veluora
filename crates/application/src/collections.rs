use domain::{Collection, CollectionId, CollectionType, ItemId};
use rusqlite::{params, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::context::AppContext;
use crate::error::{AppError, Result};
use crate::time_format::{from_rfc3339, to_rfc3339};

pub struct CollectionService;

impl CollectionService {
    /// Always creates a `Manual` collection this milestone — smart
    /// collections are schema-ready (`collections.query`) but
    /// materializing one via `SearchService` at read time isn't wired up
    /// yet.
    pub fn create(ctx: &AppContext, name: &str, description: Option<&str>) -> Result<Collection> {
        let now = OffsetDateTime::now_utc();
        let collection = Collection {
            id: CollectionId::new(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            collection_type: CollectionType::Manual,
            query: None,
            sort_mode: "added_desc".to_string(),
            cover_item_id: None,
            created_at: now,
            updated_at: now,
        };

        ctx.db
            .connection()
            .execute(
                "INSERT INTO collections (id, name, description, collection_type, query, sort_mode, cover_item_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'manual', NULL, ?4, NULL, ?5, ?5)",
                params![
                    collection.id.to_string(),
                    collection.name,
                    collection.description,
                    collection.sort_mode,
                    to_rfc3339(now),
                ],
            )
            .map_err(database::DatabaseError::from)?;

        Ok(collection)
    }

    pub fn list(ctx: &AppContext) -> Result<Vec<Collection>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, collection_type, query, sort_mode, cover_item_id, created_at, updated_at
                 FROM collections ORDER BY created_at",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map([], row_to_collection)
            .map_err(database::DatabaseError::from)?;

        let mut collections = Vec::new();
        for row in rows {
            collections.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(collections)
    }

    pub fn delete(ctx: &AppContext, id: CollectionId) -> Result<()> {
        let affected = ctx
            .db
            .connection()
            .execute(
                "DELETE FROM collections WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        if affected == 0 {
            return Err(AppError::NotFound(format!("collection {id}")));
        }
        Ok(())
    }

    pub fn add_item(ctx: &AppContext, collection_id: CollectionId, item_id: ItemId) -> Result<()> {
        ctx.db
            .connection()
            .execute(
                "INSERT OR IGNORE INTO collection_items (collection_id, item_id, added_at) VALUES (?1, ?2, ?3)",
                params![collection_id.to_string(), item_id.to_string(), to_rfc3339(OffsetDateTime::now_utc())],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    pub fn remove_item(
        ctx: &AppContext,
        collection_id: CollectionId,
        item_id: ItemId,
    ) -> Result<()> {
        ctx.db
            .connection()
            .execute(
                "DELETE FROM collection_items WHERE collection_id = ?1 AND item_id = ?2",
                params![collection_id.to_string(), item_id.to_string()],
            )
            .map_err(database::DatabaseError::from)?;
        Ok(())
    }

    pub fn list_items(ctx: &AppContext, collection_id: CollectionId) -> Result<Vec<ItemId>> {
        let conn = ctx.db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT item_id FROM collection_items WHERE collection_id = ?1 ORDER BY added_at",
            )
            .map_err(database::DatabaseError::from)?;
        let rows = stmt
            .query_map(params![collection_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                Ok(ItemId(Uuid::parse_str(&id_str).unwrap_or_default()))
            })
            .map_err(database::DatabaseError::from)?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(database::DatabaseError::from)?);
        }
        Ok(items)
    }
}

fn row_to_collection(row: &Row) -> rusqlite::Result<Collection> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let collection_type_str: String = row.get(3)?;
    let query: Option<String> = row.get(4)?;
    let sort_mode: String = row.get(5)?;
    let cover_item_id_str: Option<String> = row.get(6)?;
    let created_at_str: String = row.get(7)?;
    let updated_at_str: String = row.get(8)?;

    Ok(Collection {
        id: CollectionId(Uuid::parse_str(&id_str).unwrap_or_default()),
        name,
        description,
        collection_type: match collection_type_str.as_str() {
            "smart" => CollectionType::Smart,
            "system" => CollectionType::System,
            _ => CollectionType::Manual,
        },
        query,
        sort_mode,
        cover_item_id: cover_item_id_str.map(|s| ItemId(Uuid::parse_str(&s).unwrap_or_default())),
        created_at: from_rfc3339(&created_at_str).unwrap_or(OffsetDateTime::UNIX_EPOCH),
        updated_at: from_rfc3339(&updated_at_str).unwrap_or(OffsetDateTime::UNIX_EPOCH),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_item(ctx: &AppContext) -> ItemId {
        let item_id = ItemId::new();
        ctx.db
            .connection()
            .execute(
                "INSERT INTO media_items (id, media_type, title, rating_classification, discovered_at, updated_at)
                 VALUES (?1, 'image', 'Test', 'unrated', datetime('now'), datetime('now'))",
                params![item_id.to_string()],
            )
            .unwrap();
        item_id
    }

    #[test]
    fn create_list_and_delete_round_trip() {
        let ctx = AppContext::open_in_memory().unwrap();
        let created = CollectionService::create(&ctx, "Later", Some("watch later")).unwrap();
        assert_eq!(created.collection_type, CollectionType::Manual);

        let listed = CollectionService::list(&ctx).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Later");

        CollectionService::delete(&ctx, created.id).unwrap();
        assert!(CollectionService::list(&ctx).unwrap().is_empty());
    }

    #[test]
    fn deleting_an_unknown_collection_is_not_found() {
        let ctx = AppContext::open_in_memory().unwrap();
        let err = CollectionService::delete(&ctx, CollectionId::new()).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn add_and_remove_items() {
        let ctx = AppContext::open_in_memory().unwrap();
        let collection = CollectionService::create(&ctx, "Later", None).unwrap();
        let item_id = insert_item(&ctx);

        CollectionService::add_item(&ctx, collection.id, item_id).unwrap();
        assert_eq!(
            CollectionService::list_items(&ctx, collection.id).unwrap(),
            vec![item_id]
        );

        // Adding twice is idempotent (INSERT OR IGNORE).
        CollectionService::add_item(&ctx, collection.id, item_id).unwrap();
        assert_eq!(
            CollectionService::list_items(&ctx, collection.id)
                .unwrap()
                .len(),
            1
        );

        CollectionService::remove_item(&ctx, collection.id, item_id).unwrap();
        assert!(CollectionService::list_items(&ctx, collection.id)
            .unwrap()
            .is_empty());
    }
}

use redb::{Database, Error, ReadableDatabase, TableDefinition};
use rosu_v2::request::UserId;
use serenity::all::ChannelId;
use serenity::builder::GetMessages;
use serenity::client::Context;
use serenity::model::id::MessageId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::utils::osu_utils::map_url_from_msg;

// Define the table schema: Discord User ID (u64) -> osu! User ID (u32)
const USER_ID_TABLE: TableDefinition<u64, u32> = TableDefinition::new("osu_users_by_id");
const USER_NAME_TABLE: TableDefinition<u64, String> = TableDefinition::new("osu_users_by_name");

#[derive(Clone)]
pub struct UserDb {
    db: Arc<Database>,
}

impl UserDb {
    /// Opens or creates the database file on disk.
    pub fn new(file_path: &str) -> Result<Self, Error> {
        let db = Database::create(file_path)?;

        // Ensure the table exists on startup
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(USER_ID_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Save or update a Discord user's saved osu! user ID
    pub fn set_user_id(&self, discord_id: u64, osu_id: UserId) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        match osu_id {
            UserId::Id(id) => {
                let mut table = write_txn.open_table(USER_ID_TABLE)?;
                table.insert(discord_id, id)?;
            }
            UserId::Name(name) => {
                let mut table = write_txn.open_table(USER_NAME_TABLE)?;
                table.insert(discord_id, name.to_string())?;
            }
        };
        write_txn.commit()?;
        Ok(())
    }

    /// Fetch a user's saved osu! user ID (returns None if not linked)
    pub fn get_user_id(&self, discord_id: u64) -> Result<Option<UserId>, Error> {
        let read_txn = self.db.begin_read()?;
        let user_id_table = read_txn.open_table(USER_ID_TABLE)?;

        if let Some(guard) = user_id_table.get(discord_id)? {
            return Ok(Some(UserId::Id(guard.value())));
        }

        let user_name_table = read_txn.open_table(USER_NAME_TABLE)?;
        if let Some(guard) = user_name_table.get(discord_id)? {
            return Ok(Some(UserId::Name(guard.value().into())));
        }

        Ok(None)
    }

    /// Remove a user's link if they want to unlink
    pub fn remove_user_id(&self, discord_id: u64) -> Result<bool, Error> {
        let write_txn = self.db.begin_write()?;
        let removed_id = write_txn
            .open_table(USER_ID_TABLE)?
            .remove(discord_id)?
            .is_some();
        let removed_name = write_txn
            .open_table(USER_NAME_TABLE)?
            .remove(discord_id)?
            .is_some();
        write_txn.commit()?;
        Ok(removed_id || removed_name)
    }
}

#[derive(Clone)]
pub struct ChannelMapDb {
    maps: Arc<Mutex<HashMap<ChannelId, String>>>,
    first_checked_msg: Arc<Mutex<HashMap<ChannelId, MessageId>>>,
    ctx: Context,
}

impl ChannelMapDb {
    pub fn new(ctx: Context) -> Self {
        Self {
            maps: Arc::new(Mutex::new(HashMap::new())),
            first_checked_msg: Arc::new(Mutex::new(HashMap::new())),
            ctx,
        }
    }

    pub fn set_channel_map(
        &self,
        channel_id: ChannelId,
        map_url: String,
        msg_id: Option<MessageId>,
    ) {
        self.maps.lock().unwrap().insert(channel_id, map_url);
        if self
            .first_checked_msg
            .lock()
            .unwrap()
            .get(&channel_id)
            .is_none()
            && let Some(msg_id) = msg_id
        {
            self.set_last_checked_msg(channel_id, msg_id);
        }
    }

    pub async fn get_channel_map(
        &self,
        channel_id: ChannelId,
    ) -> Option<String> {
        if let Some(map_url) = self.maps.lock().unwrap().get(&channel_id) {
            return Some(map_url.to_string());
        };

        let last_checked_msg_id = self
            .first_checked_msg
            .lock()
            .unwrap()
            .get(&channel_id)
            .cloned();

        let builder = last_checked_msg_id
            .map(|msg_id| GetMessages::new().before(msg_id).limit(50))
            .unwrap_or(GetMessages::new().limit(50));

        let msgs = channel_id.messages(&self.ctx, builder).await.ok()?;

        if msgs.is_empty() {
            return None;
        }

        let url = msgs.iter().find_map(map_url_from_msg);

        self.set_last_checked_msg(channel_id, msgs.last().unwrap().id);
        url
    }

    fn set_last_checked_msg(&self, channel_id: ChannelId, msg_id: MessageId) {
        self.first_checked_msg
            .lock()
            .unwrap()
            .insert(channel_id, msg_id);
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_db_operations() {
        let test_db_file = "test_users.redb";

        // Clean up old test file if it exists
        let _ = std::fs::remove_file(test_db_file);

        // 1. Initialize
        let db = UserDb::new(test_db_file).expect("Failed to create test db");

        let discord_id = 123456789012345678u64;
        let osu_id = UserId::Id(9255551u32); // Example osu! ID (mrekk = 9255551)

        // 2. Test Get (should be None initially)
        let initial_get = db.get_user_id(discord_id).unwrap();
        assert_eq!(initial_get, None);

        // 3. Test Set
        db.set_user_id(discord_id, osu_id.clone())
            .expect("Failed to write user ID");

        // 4. Test Get (should return Some(9255551))
        let retrieved = db.get_user_id(discord_id).unwrap();
        assert_eq!(retrieved, Some(osu_id));

        // 5. Test Update
        let new_osu_id = UserId::Id(2u32); // Cookiezi = 2
        db.set_user_id(discord_id, new_osu_id.clone())
            .expect("Failed to update user ID");
        assert_eq!(db.get_user_id(discord_id).unwrap(), Some(new_osu_id));

        // 6. Test Remove
        let removed = db.remove_user_id(discord_id).unwrap();
        assert!(removed);
        assert_eq!(db.get_user_id(discord_id).unwrap(), None);

        // Cleanup test file
        let _ = std::fs::remove_file(test_db_file);
    }
}

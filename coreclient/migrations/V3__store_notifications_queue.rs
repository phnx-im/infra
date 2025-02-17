use crate::store::StoreNotification;

pub fn migration() -> String {
    StoreNotification::CREATE_TABLE_STATEMENT.to_string()
}

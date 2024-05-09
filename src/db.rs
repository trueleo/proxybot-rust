use sqlite::State;

pub const CREATE_STATEMENT: &str = "CREATE TABLE IF NOT EXISTS forwards (message_id INTEGER PRIMARY KEY, user_id INTEGER, dm_message_id INTEGER);";

pub fn get_from_message_id(
    db: &sqlite::ConnectionThreadSafe,
    message_id: impl Into<i64>,
) -> Result<Option<(i64, i64)>, anyhow::Error> {
    let mut query =
        db.prepare("select user_id, dm_message_id from forwards where message_id = ?")?;
    query.bind((1, message_id.into()))?;
    let res = if let Ok(State::Row) = query.next() {
        Some((
            query.read::<i64, _>("user_id").unwrap(),
            query.read::<i64, _>("dm_message_id").unwrap(),
        ))
    } else {
        None
    };
    Ok(res)
}

pub struct InsertValues {
    pub message_id: i64,
    pub user_id: i64,
    pub dm_message_id: i64,
}

pub fn insert_into(
    db: &sqlite::ConnectionThreadSafe,
    values: InsertValues,
) -> Result<(), anyhow::Error> {
    db.execute(format!(
        "INSERT INTO forwards VALUES ({}, {}, {})",
        values.message_id, values.user_id, values.dm_message_id
    ))?;
    Ok(())
}

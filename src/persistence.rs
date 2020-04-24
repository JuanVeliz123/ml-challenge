extern crate rusqlite;

use rusqlite::{Connection, Result, NO_PARAMS};

pub struct EmailEntity {
    pub date_received: String,
    pub from: String,
    pub subject: String,
}

pub fn persist_emails(emails: Vec<EmailEntity>, db_name: String) -> Result<()> {
    let mut conn = Connection::open(db_name)?;
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS emails (
            id INTEGER PRIMARY KEY,
            msg_from TEXT NOT NULL,
            subject TEXT NOT NULL,
            date_received TEXT NOT NULL
        )",
        NO_PARAMS
    ) {
        Ok(_) => (),
        Err(err) => panic!("{:?}", err),
    };

    let tx = conn.transaction()?;
    for email in emails {
        tx.execute(
            "INSERT INTO emails (msg_from, subject, date_received) VALUES (?1, ?2, ?3)",
            &[email.from, email.subject, email.date_received]
        )?;
    }
    tx.commit()
}
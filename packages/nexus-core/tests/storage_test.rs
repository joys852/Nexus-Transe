use nexus_core::models::MessageRole;
use nexus_core::storage::sqlite::SqliteStore;
use nexus_core::storage::SessionRepository;
use tempfile::tempdir;

#[tokio::test]
async fn create_session_and_append_message() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");
    let store = SqliteStore::connect(&db).await.unwrap();
    let session = store.create_session(None, Some("test")).await.unwrap();
    store
        .append_message(session.id, MessageRole::User, "hello", None)
        .await
        .unwrap();
    let messages = store.list_messages(session.id, 10).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "hello");
}

#[tokio::test]
async fn replace_session_messages() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("test.db");
    let store = SqliteStore::connect(&db).await.unwrap();
    let session = store.create_session(None, Some("t")).await.unwrap();
    store
        .append_message(session.id, MessageRole::User, "old", None)
        .await
        .unwrap();
    store
        .replace_session_messages(
            session.id,
            &[
                (MessageRole::User, "a".into()),
                (MessageRole::Assistant, "b".into()),
            ],
        )
        .await
        .unwrap();
    let messages = store.list_messages(session.id, 10).await.unwrap();
    assert_eq!(messages.len(), 2);
}

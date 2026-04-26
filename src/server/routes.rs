use crate::server::{
    Databases, Message, SearchIndex, ThreadResponse,
    models::{Conversation, convo_key, msg_prefix},
};
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    App, HttpResponse, HttpServer, Responder,
    middleware::Logger,
    web::{self, Data, Query},
};
use lmdb::{Cursor, Transaction};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub dbs: Databases,
    pub index: Arc<RwLock<SearchIndex>>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

async fn search(state: Data<AppState>, query: Query<SearchQuery>) -> impl Responder {
    let q = query.q.clone().unwrap_or_default();
    let limit = query.limit.unwrap_or(200);
    let index = state.index.read().await;
    let result = index.search(&q, limit);
    HttpResponse::Ok().json(result)
}

async fn get_thread(state: Data<AppState>, path: web::Path<String>) -> impl Responder {
    let uuid = path.into_inner();

    // Fetch conversation
    let convo: Conversation = {
        let txn = match state.dbs.convos.env.begin_ro_txn() {
            Ok(t) => t,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
        };
        let key = convo_key(&uuid);
        let bytes = match txn.get(*state.dbs.convos.db, &key) {
            Ok(b) => b,
            Err(_) => return HttpResponse::NotFound().body("conversation not found"),
        };
        match bitcode::decode(bytes) {
            Ok(c) => c,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
        }
    };

    // Fetch messages via prefix scan
    let messages: Vec<Message> = {
        let txn = match state.dbs.messages.env.begin_ro_txn() {
            Ok(t) => t,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
        };
        let mut cursor = match txn.open_ro_cursor(*state.dbs.messages.db) {
            Ok(c) => c,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
        };
        let prefix = msg_prefix(&uuid);
        let mut msgs = Vec::new();
        for (key, val) in cursor.iter_from(prefix.as_bytes()) {
            let key_str = std::str::from_utf8(key).unwrap_or("");
            if !key_str.starts_with(&prefix) {
                break;
            }
            if let Ok(msg) = bitcode::decode::<Message>(val) {
                msgs.push(msg);
            }
        }
        msgs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        msgs
    };

    HttpResponse::Ok().json(ThreadResponse { convo, messages })
}

pub async fn start_server(dbs: Databases, index: Arc<RwLock<SearchIndex>>, bind: String) -> std::io::Result<()> {
    let state = Data::new(AppState { dbs, index });
    
    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
        .wrap(cors)
        .wrap(Logger::default())
        .app_data(state.clone())
        .route("/api/search", web::get().to(search))
        .route("/api/thread/{uuid}", web::get().to(get_thread))
        .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(&bind)?
    .run()
    .await
}

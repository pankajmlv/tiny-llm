use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Serialize)]
struct Todo {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Deserialize)]
struct CreateTodo {
    title: String,
}

#[derive(Deserialize)]
struct UpdateTodo {
    title: Option<String>,
    done: Option<bool>,
}

// In-memory store; data is lost when the server stops.
#[derive(Default)]
struct Db {
    todos: HashMap<u64, Todo>,
    next_id: u64,
}

type AppState = Arc<Mutex<Db>>;

#[tokio::main]
async fn main() {
    let state = AppState::default();

    let app = Router::new()
        .route("/health", get(health))
        .route("/todos", get(list_todos).post(create_todo))
        .route(
            "/todos/:id",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("Listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn list_todos(State(db): State<AppState>) -> Json<Vec<Todo>> {
    let db = db.lock().unwrap();
    let mut todos: Vec<Todo> = db.todos.values().cloned().collect();
    todos.sort_by_key(|t| t.id);
    Json(todos)
}

async fn create_todo(
    State(db): State<AppState>,
    Json(input): Json<CreateTodo>,
) -> (StatusCode, Json<Todo>) {
    let mut db = db.lock().unwrap();
    db.next_id += 1;
    let todo = Todo {
        id: db.next_id,
        title: input.title,
        done: false,
    };
    db.todos.insert(todo.id, todo.clone());
    (StatusCode::CREATED, Json(todo))
}

async fn get_todo(
    State(db): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Todo>, StatusCode> {
    let db = db.lock().unwrap();
    db.todos.get(&id).cloned().map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn update_todo(
    State(db): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<UpdateTodo>,
) -> Result<Json<Todo>, StatusCode> {
    let mut db = db.lock().unwrap();
    let todo = db.todos.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(title) = input.title {
        todo.title = title;
    }
    if let Some(done) = input.done {
        todo.done = done;
    }
    Ok(Json(todo.clone()))
}

async fn delete_todo(State(db): State<AppState>, Path(id): Path<u64>) -> StatusCode {
    let mut db = db.lock().unwrap();
    if db.todos.remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::fs;
use tower_http::services::ServeDir;

mod assets;
mod models;
mod telegram;
mod templates;

use crate::models::{now_secs, now_stamp, Expense, Ledger, CATEGORIES};
use crate::templates::{AppTemplate, RegionTemplate};

struct AppState {
    expenses: Mutex<Vec<Expense>>,
    counter: AtomicU64,
}

type Shared = Arc<AppState>;

fn snapshot(state: &AppState) -> Vec<Expense> {
    state.expenses.lock().unwrap().clone()
}

fn toast_header(message: &str) -> String {
    serde_json::json!({ "toast": { "message": message } }).to_string()
}

fn region_response(message: &str, expenses: &[Expense]) -> impl IntoResponse {
    (
        [("HX-Trigger", toast_header(message))],
        RegionTemplate {
            ledger: Ledger::from_expenses(expenses),
        },
    )
}

fn reject(message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        [("HX-Trigger", toast_header(message))],
    )
        .into_response()
}

fn receipt_extension(content_type: &str) -> Option<&'static str> {
    match content_type.to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "image/heic" => Some("heic"),
        "image/heif" => Some("heif"),
        "application/pdf" => Some("pdf"),
        _ => None,
    }
}

async fn persist(expenses: &[Expense]) {
    match serde_json::to_string_pretty(expenses) {
        Ok(json) => {
            if let Err(err) = fs::write("data/expenses.json", json).await {
                eprintln!("Could not write data/expenses.json: {}", err);
            }
        }
        Err(err) => eprintln!("Could not serialise expenses: {}", err),
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    assets::init();

    fs::create_dir_all("data/receipts").await.ok();

    let data = fs::read_to_string("data/expenses.json")
        .await
        .unwrap_or_else(|_| "[]".to_string());
    let expenses: Vec<Expense> = serde_json::from_str(&data).unwrap_or_else(|err| {
        eprintln!("Could not parse data/expenses.json: {}", err);
        Vec::new()
    });

    let state: Shared = Arc::new(AppState {
        expenses: Mutex::new(expenses),
        counter: AtomicU64::new(0),
    });

    let app = Router::new()
        .route("/", get(render_app))
        .route("/region", get(render_region))
        .route("/api/expenses", post(create_expense))
        .route("/api/expenses/:id/delete", post(delete_expense))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/receipts", ServeDir::new("data/receipts"))
        .layer(DefaultBodyLimit::max(15 * 1024 * 1024))
        .layer(axum::middleware::from_fn(assets::cache_headers))
        .with_state(state);

    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", bind, port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Mini App is running on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn render_app(State(state): State<Shared>) -> impl IntoResponse {
    AppTemplate {
        ledger: Ledger::from_expenses(&snapshot(&state)),
        categories: &CATEGORIES,
    }
}

async fn render_region(State(state): State<Shared>) -> impl IntoResponse {
    RegionTemplate {
        ledger: Ledger::from_expenses(&snapshot(&state)),
    }
}

async fn create_expense(State(state): State<Shared>, mut multipart: Multipart) -> Response {
    let mut category = String::new();
    let mut date = String::new();
    let mut description = String::new();
    let mut amount = String::new();
    let mut receipt: Option<(&'static str, Vec<u8>)> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => return reject("Upload failed. Please try again."),
        };
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "category" => category = field.text().await.unwrap_or_default(),
            "date" => date = field.text().await.unwrap_or_default(),
            "description" => description = field.text().await.unwrap_or_default(),
            "amount" => amount = field.text().await.unwrap_or_default(),
            "receipt" => {
                let ext = receipt_extension(field.content_type().unwrap_or_default());
                let bytes = match field.bytes().await {
                    Ok(bytes) => bytes,
                    Err(_) => return reject("The receipt is too large or could not be read."),
                };
                if !bytes.is_empty() {
                    match ext {
                        Some(ext) => receipt = Some((ext, bytes.to_vec())),
                        None => return reject("The receipt must be an image or a PDF."),
                    }
                }
            }
            _ => {}
        }
    }

    let Some(cat) = models::category(category.trim()) else {
        return reject("Choose an expense type.");
    };

    let description = description.trim().to_string();
    if description.is_empty() || description.chars().count() > 120 {
        return reject("Enter a short description.");
    }

    let Some(pence) = models::parse_pence(&amount) else {
        return reject("Enter a valid amount.");
    };

    let date = date.trim().to_string();
    if !models::is_valid_date(&date) {
        return reject("Pick a valid date.");
    }

    let Some((ext, bytes)) = receipt else {
        return reject("Please attach a receipt.");
    };

    let id = format!(
        "exp_{}_{}",
        now_secs(),
        state.counter.fetch_add(1, Ordering::Relaxed)
    );
    let file_name = format!("{}.{}", id, ext);

    if fs::create_dir_all("data/receipts").await.is_err()
        || fs::write(format!("data/receipts/{}", file_name), bytes)
            .await
            .is_err()
    {
        return reject("Could not store the receipt.");
    }

    let expense = Expense {
        id,
        category: cat.key.to_string(),
        date,
        description,
        pence,
        receipt: file_name,
        created_at: now_stamp(),
    };

    let list = {
        let mut guard = state.expenses.lock().unwrap();
        guard.push(expense.clone());
        guard.clone()
    };
    persist(&list).await;

    let alert = expense.clone();
    tokio::spawn(async move {
        telegram::send_expense_alert(&alert).await;
    });

    region_response("Expense saved", &list).into_response()
}

async fn delete_expense(State(state): State<Shared>, Path(id): Path<String>) -> impl IntoResponse {
    let (list, removed) = {
        let mut guard = state.expenses.lock().unwrap();
        let position = guard.iter().position(|e| e.id == id);
        let removed = position.map(|index| guard.remove(index));
        (guard.clone(), removed)
    };

    let message = match removed {
        Some(expense) => {
            if expense.has_receipt() {
                let _ = fs::remove_file(format!("data/receipts/{}", expense.receipt)).await;
            }
            persist(&list).await;
            "Expense deleted"
        }
        None => "This expense was already removed",
    };

    region_response(message, &list)
}
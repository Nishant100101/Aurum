use crate::models::Expense;
use reqwest::Client;
use serde_json::json;
use std::env;

struct Credentials {
    token: String,
    chat_id: String,
    bot_username: String,
    app_name: String,
}

fn dev_preview() -> bool {
    env::var("DEV_PREVIEW")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn credentials() -> Option<Credentials> {
    let token = env::var("TELEGRAM_BOT_TOKEN").ok()?;
    let chat_id = env::var("TELEGRAM_GROUP_ID").ok()?;
    if token.is_empty() || chat_id.is_empty() {
        return None;
    }
    Some(Credentials {
        token,
        chat_id,
        bot_username: env::var("TELEGRAM_BOT_USERNAME")
            .unwrap_or_default()
            .replace('@', ""),
        app_name: env::var("TELEGRAM_MINI_APP_NAME").unwrap_or_default(),
    })
}

fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn deep_link(creds: &Credentials, expense_id: &str) -> String {
    if creds.app_name.is_empty() {
        format!("https://t.me/{}?startapp={}", creds.bot_username, expense_id)
    } else {
        format!(
            "https://t.me/{}/{}?startapp={}",
            creds.bot_username, creds.app_name, expense_id
        )
    }
}

fn message(expense: &Expense) -> String {
    let receipt = if expense.has_receipt() { "Attached ✓" } else { "None" };
    format!(
        "<b>✦ AURUM</b>  ·  <i>Expense Recorder</i>\n\
        ━━━━━━━━━━━━━━\n\
        🧾 <b>NEW EXPENSE RECORDED</b>\n\n\
        • <b>Type:</b> {}\n\
        • <b>Date:</b> {}\n\
        • <b>Description:</b> {}\n\
        • <b>Amount:</b> <code>{}</code>\n\
        • <b>Receipt:</b> {}",
        esc(expense.category_label()),
        esc(&expense.date_long()),
        esc(&expense.description),
        esc(&expense.amount_text()),
        receipt
    )
}

pub async fn send_expense_alert(expense: &Expense) {
    if dev_preview() {
        println!("DEV_PREVIEW: skipped Telegram alert for {}", expense.id);
        return;
    }

    let Some(creds) = credentials() else {
        eprintln!("Telegram credentials missing in .env");
        return;
    };

    let payload = json!({
        "chat_id": creds.chat_id,
        "text": message(expense),
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
        "reply_markup": {
            "inline_keyboard": [[
                { "text": "📂 Open Expense", "url": deep_link(&creds, &expense.id) }
            ]]
        }
    });

    let endpoint = format!("https://api.telegram.org/bot{}/sendMessage", creds.token);

    match Client::new().post(&endpoint).json(&payload).send().await {
        Ok(res) if res.status().is_success() => {
            println!("Alert sent for expense {}", expense.id);
        }
        Ok(res) => {
            eprintln!("Telegram API error: {}", res.text().await.unwrap_or_default());
        }
        Err(err) => eprintln!("Failed to send Telegram request: {}", err),
    }
}

pub async fn send_welcome() -> bool {
    let Some(creds) = credentials() else {
        eprintln!("Telegram credentials missing in .env");
        return false;
    };

    let text = "<b>✦ A U R U M</b>\n\
        <i>Expense Recorder</i>\n\n\
        ━━━━━━━━━━━━━━\n\n\
        Record every business expense in under a minute.\n\n\
        <b>1</b>  Choose the expense type\n\
        <b>2</b>  Pick the date\n\
        <b>3</b>  Add the description and amount\n\
        <b>4</b>  Attach your receipt\n\n\
        Everything is saved, organised and ready for your accounts.\n\n\
        👇 <b>Tap below to begin</b>";

    let payload = json!({
        "chat_id": creds.chat_id,
        "text": text,
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
        "reply_markup": {
            "inline_keyboard": [[
                { "text": "✦ Open Aurum Expenses", "url": deep_link(&creds, "home") }
            ]]
        }
    });

    let endpoint = format!("https://api.telegram.org/bot{}/sendMessage", creds.token);

    match Client::new().post(&endpoint).json(&payload).send().await {
        Ok(res) if res.status().is_success() => true,
        Ok(res) => {
            eprintln!("Telegram API error: {}", res.text().await.unwrap_or_default());
            false
        }
        Err(err) => {
            eprintln!("Failed to send Telegram request: {}", err);
            false
        }
    }
}
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
pub struct Category {
    pub key: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub tone: &'static str,
}

pub static CATEGORIES: [Category; 8] = [
    Category { key: "business-entertainment", label: "Business Entertainment", icon: "i-utensils", tone: "hot" },
    Category { key: "personal-entertainment", label: "Personal Entertainment", icon: "i-film", tone: "violet" },
    Category { key: "postage", label: "Postage", icon: "i-mail", tone: "teal" },
    Category { key: "general-expenses", label: "General Expenses", icon: "i-briefcase", tone: "secondary" },
    Category { key: "printing-stationery", label: "Printing & Stationery", icon: "i-printer", tone: "warning" },
    Category { key: "it-software-consumables", label: "IT Software & Consumables", icon: "i-monitor", tone: "primary" },
    Category { key: "repairs-maintenance", label: "Repairs & Maintenance", icon: "i-wrench", tone: "danger" },
    Category { key: "travel-subsistence", label: "Travel & Subsistence", icon: "i-plane", tone: "sky" },
];

pub fn category(key: &str) -> Option<&'static Category> {
    CATEGORIES.iter().find(|c| c.key == key)
}

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const MONTHS_LONG: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Expense {
    pub id: String,
    pub category: String,
    pub date: String,
    pub description: String,
    pub pence: i64,
    #[serde(default)]
    pub receipt: String,
    #[serde(default)]
    pub created_at: String,
}

impl Expense {
    fn meta(&self) -> Option<&'static Category> {
        category(&self.category)
    }

    pub fn category_label(&self) -> &'static str {
        self.meta().map(|c| c.label).unwrap_or("General Expenses")
    }

    pub fn category_icon(&self) -> &'static str {
        self.meta().map(|c| c.icon).unwrap_or("i-briefcase")
    }

    pub fn tone(&self) -> &'static str {
        self.meta().map(|c| c.tone).unwrap_or("secondary")
    }

    pub fn amount_text(&self) -> String {
        money(self.pence)
    }

    pub fn date_short(&self) -> String {
        match parse_date(&self.date) {
            Some((y, m, d, _)) => format!("{} {} {}", d, MONTHS_SHORT[(m - 1) as usize], y),
            None => self.date.clone(),
        }
    }

    pub fn date_long(&self) -> String {
        long_date(&self.date)
    }

    pub fn recorded_text(&self) -> String {
        let Some((date, time)) = self.created_at.split_once(' ') else {
            return self.created_at.clone();
        };
        let Some((y, m, d, _)) = parse_date(date) else {
            return self.created_at.clone();
        };
        let hhmm = time.get(..5).unwrap_or(time);
        format!("{} {} {}, {}", d, MONTHS_SHORT[(m - 1) as usize], y, hhmm)
    }
    
    pub fn has_receipt(&self) -> bool {
        !self.receipt.is_empty()
    }

    pub fn receipt_url(&self) -> String {
        format!("/receipts/{}", self.receipt)
    }

    pub fn receipt_is_image(&self) -> bool {
        !self.receipt.to_lowercase().ends_with(".pdf")
    }

    pub fn search_blob(&self) -> String {
        format!(
            "{} {} {} {}",
            self.description,
            self.category_label(),
            self.amount_text(),
            self.date_short()
        )
        .to_lowercase()
    }
}

pub struct DayGroup {
    pub date: String,
    pub label: String,
    pub total: String,
    pub pence: i64,
    pub items: Vec<Expense>,
}

pub struct CategoryStat {
    pub key: &'static str,
    pub label: &'static str,
    pub count: usize,
}

pub struct BreakdownItem {
    pub label: &'static str,
    pub tone: &'static str,
    pub percent: u32,
    pub amount: String,
}

pub struct Ledger {
    pub groups: Vec<DayGroup>,
    pub count: usize,
    pub month_label: String,
    pub month_total: String,
    pub month_count: usize,
    pub all_total: String,
    pub stats: Vec<CategoryStat>,
    pub breakdown: Vec<BreakdownItem>,
}

impl Ledger {
    pub fn from_expenses(expenses: &[Expense]) -> Self {
        let mut sorted = expenses.to_vec();
        sorted.sort_by(|a, b| {
            b.date
                .cmp(&a.date)
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| b.id.cmp(&a.id))
        });

        let stamp = now_stamp();
        let month_prefix = stamp[..7].to_string();
        let month_index = stamp[5..7].parse::<usize>().unwrap_or(1).clamp(1, 12);
        let month_label = format!("{} {}", MONTHS_LONG[month_index - 1], &stamp[..4]);

        let mut groups: Vec<DayGroup> = Vec::new();
        for expense in sorted {
            let same = groups.last().is_some_and(|g| g.date == expense.date);
            if same {
                let group = groups.last_mut().unwrap();
                group.pence += expense.pence;
                group.items.push(expense);
            } else {
                groups.push(DayGroup {
                    date: expense.date.clone(),
                    label: long_date(&expense.date),
                    total: String::new(),
                    pence: expense.pence,
                    items: vec![expense],
                });
            }
        }
        for group in groups.iter_mut() {
            group.total = money(group.pence);
        }

        let all_pence: i64 = expenses.iter().map(|e| e.pence).sum();
        let month_pence: i64 = expenses
            .iter()
            .filter(|e| e.date.starts_with(&month_prefix))
            .map(|e| e.pence)
            .sum();
        let month_count = expenses
            .iter()
            .filter(|e| e.date.starts_with(&month_prefix))
            .count();

        let mut stats = Vec::new();
        for cat in CATEGORIES.iter() {
            let count = expenses.iter().filter(|e| e.category == cat.key).count();
            if count > 0 {
                stats.push(CategoryStat {
                    key: cat.key,
                    label: cat.label,
                    count,
                });
            }
        }

        let mut totals: Vec<(&'static Category, i64)> = Vec::new();
        for cat in CATEGORIES.iter() {
            let sum: i64 = expenses
                .iter()
                .filter(|e| e.category == cat.key && e.date.starts_with(&month_prefix))
                .map(|e| e.pence)
                .sum();
            if sum > 0 {
                totals.push((cat, sum));
            }
        }
        totals.sort_by(|a, b| b.1.cmp(&a.1));

        let breakdown = totals
            .into_iter()
            .take(4)
            .map(|(cat, sum)| BreakdownItem {
                label: cat.label,
                tone: cat.tone,
                percent: (((sum * 100) / month_pence.max(1)) as u32).max(2),
                amount: money(sum),
            })
            .collect();

        Ledger {
            groups,
            count: expenses.len(),
            month_label,
            month_total: money(month_pence),
            month_count,
            all_total: money(all_pence),
            stats,
            breakdown,
        }
    }
}

pub fn money(pence: i64) -> String {
    let pounds = (pence / 100).to_string();
    let rest = pence % 100;
    let mut grouped = String::new();
    for (index, ch) in pounds.chars().enumerate() {
        if index > 0 && (pounds.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("£{}.{:02}", grouped, rest)
}

pub fn parse_pence(input: &str) -> Option<i64> {
    let value = input.trim();
    let (whole, frac) = match value.split_once('.') {
        Some((w, f)) => (w, f),
        None => (value, ""),
    };
    if whole.is_empty() || whole.len() > 7 || frac.len() > 2 {
        return None;
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let pounds: i64 = whole.parse().ok()?;
    let pence: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        _ => frac.parse::<i64>().ok()?,
    };
    let total = pounds * 100 + pence;
    if total > 0 {
        Some(total)
    } else {
        None
    }
}

pub fn is_valid_date(value: &str) -> bool {
    match parse_date(value) {
        Some((year, _, _, days)) => {
            let today = today_days();
            year >= 2000 && days <= today + 1 && days >= today - 3650
        }
        None => false,
    }
}

fn long_date(value: &str) -> String {
    match parse_date(value) {
        Some((y, m, d, days)) => {
            let weekday = WEEKDAYS[(days + 4).rem_euclid(7) as usize];
            format!("{} {} {} {}", weekday, d, MONTHS_SHORT[(m - 1) as usize], y)
        }
        None => value.to_string(),
    }
}

fn parse_date(value: &str) -> Option<(i64, i64, i64, i64)> {
    if !value.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return None;
    }
    let mut parts = value.split('-');
    let year = parts.next()?;
    let month = parts.next()?;
    let day = parts.next()?;
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    let y: i64 = year.parse().ok()?;
    let m: i64 = month.parse().ok()?;
    let d: i64 = day.parse().ok()?;
    if !(1..=12).contains(&m) || d < 1 {
        return None;
    }
    let days = days_from_civil(y, m, d);
    if civil_from_days(days) == (y, m, d) {
        Some((y, m, d, days))
    } else {
        None
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn today_days() -> i64 {
    now_secs().div_euclid(86_400)
}

pub fn now_stamp() -> String {
    let secs = now_secs();
    let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
    let rem = secs.rem_euclid(86_400);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}
use crate::assets;
use crate::models::{Category, Currency, Ledger};
use askama::Template;

#[derive(Template)]
#[template(path = "layout/app.html")]
pub struct AppTemplate {
    pub ledger: Ledger,
    pub categories: &'static [Category],
    pub currencies: &'static [Currency],
}

impl AppTemplate {
    pub fn asset(&self, path: &str) -> String {
        assets::url(path)
    }

    pub fn build(&self) -> &'static str {
        assets::build()
    }
}

#[derive(Template)]
#[template(path = "components/region.html")]
pub struct RegionTemplate {
    pub ledger: Ledger,
}
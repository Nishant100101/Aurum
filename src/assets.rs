use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

static FINGERPRINTS: OnceLock<HashMap<String, String>> = OnceLock::new();
static BUILD: OnceLock<String> = OnceLock::new();

pub fn init() {
    let mut map = HashMap::new();
    collect(Path::new("assets"), &mut map);

    let mut entries: Vec<_> = map.iter().collect();
    entries.sort();
    let mut build = Fnv::new();
    for (path, fp) in entries {
        build.write(path.as_bytes());
        build.write(fp.as_bytes());
    }
    if let Ok(exe) = std::env::current_exe().and_then(std::fs::read) {
        build.write(&exe);
    }
    let _ = BUILD.set(build.short());
    let _ = FINGERPRINTS.set(map);
}

pub fn url(path: &str) -> String {
    match FINGERPRINTS.get().and_then(|m| m.get(path)) {
        Some(fp) => format!("{path}?v={fp}"),
        None => path.to_string(),
    }
}

pub fn build() -> &'static str {
    BUILD.get().map(String::as_str).unwrap_or("dev")
}

pub async fn cache_headers(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let current = req
        .uri()
        .query()
        .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("v=")))
        .is_some_and(|v| FINGERPRINTS.get().and_then(|m| m.get(&path)).is_some_and(|fp| fp == v));

    let mut res = next.run(req).await;
    let ok = res.status().is_success() || res.status() == StatusCode::NOT_MODIFIED;

    let value = if current && ok {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    res.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    res
}

fn collect(dir: &Path, map: &mut HashMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, map);
        } else if let Ok(bytes) = std::fs::read(&path) {
            let mut h = Fnv::new();
            h.write(&bytes);
            let key = format!("/{}", path.to_string_lossy().replace('\\', "/"));
            map.insert(key, h.short());
        }
    }
}

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn short(&self) -> String {
        format!("{:016x}", self.0)[..12].to_string()
    }
}
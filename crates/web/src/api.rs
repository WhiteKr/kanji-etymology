//! 런타임 데이터 로딩 — build-data 산출 JSON을 fetch한다.
//!
//! ## 데이터 서빙 방식 (선택 근거)
//!
//! build-data 출력물을 `crates/web/assets/data/`에 생성하고
//! dx의 asset 시스템(`asset!` 폴더 애셋)으로 번들한다.
//!
//! - `dx serve` 개발 서버와 `dx bundle` 정적 산출물(GitHub Pages) 모두
//!   **같은 코드 경로**로 서빙되므로 별도 복사 스크립트나 서버 설정이 필요 없다.
//! - 해시 접미사를 끄면(`with_hash_suffix(false)`) URL이 `/assets/data/...`로
//!   안정되어 curl 검증·디버깅이 쉽다. 데이터는 배포마다 바뀔 수 있으므로
//!   캐시 버스팅이 필요해지면 M8(CI/CD)에서 재검토한다.
//! - GitHub Pages 프로젝트 페이지(base_path 하위 배포)에서도 dx가 Asset 경로에
//!   base_path를 반영해 주므로 그대로 동작한다.
//!
//! 생성 명령: `cargo run -p build-data -- --content-dir content --out-dir crates/web/assets/data`
//! (산출물은 커밋하지 않는다 — 루트 `.gitignore` 참조)

use std::collections::HashMap;

use dioxus::prelude::*;
use kanji_schema::{KanjiEntry, RadicalEntry};
use serde::Deserialize;

/// build-data 출력 폴더 전체를 번들하는 폴더 애셋.
/// 폴더 내부 파일들은 원래 상대 경로를 유지한다.
pub static DATA_DIR: Asset = asset!(
    "/assets/data",
    AssetOptions::builder().with_hash_suffix(false)
);

/// `kanji/{한자}.json` — KanjiEntry 전 필드 + 어원 서술 마크다운 원문.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct KanjiDetail {
    #[serde(flatten)]
    pub entry: KanjiEntry,
    /// 어원 서술("왜 이 모양이 되었나") 마크다운 원문.
    pub body_markdown: String,
}

/// `kanji-list.json`의 항목 하나 (랜딩 그리드용 요약).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct KanjiSummary {
    pub character: String,
    pub korean_reading: String,
    pub meanings: Vec<String>,
    pub jlpt: Option<String>,
    pub stroke_count: Option<u32>,
    pub grade: Option<u8>,
}

/// `radicals/{부수}.json` — RadicalEntry 전 필드 + 어원 서술 마크다운 원문.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RadicalDetail {
    #[serde(flatten)]
    pub entry: RadicalEntry,
    /// 부수 어원 서술 마크다운 원문.
    pub body_markdown: String,
}

/// `radicals-list.json`의 항목 하나 (부수 인덱스 페이지용 요약).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RadicalSummary {
    pub radical: String,
    pub name: String,
    pub meaning: String,
    pub stroke_count: u32,
    /// 이 부수를 부품으로 갖는 등재 한자 수.
    pub kanji_count: usize,
}

/// 데이터 fetch 실패 종류. 404(콘텐츠 없음)는 별도 안내를 위해 구분한다.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchError {
    /// 해당 데이터 파일이 없음 (아직 등재되지 않은 한자 등).
    NotFound,
    /// 그 외 HTTP 오류 상태.
    Status(u16),
    /// 네트워크 또는 역직렬화 실패.
    Other(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::NotFound => write!(f, "데이터를 찾을 수 없습니다 (404)"),
            FetchError::Status(code) => write!(f, "서버 오류 (HTTP {code})"),
            FetchError::Other(msg) => write!(f, "불러오기 실패: {msg}"),
        }
    }
}

/// 상대 경로 JSON을 fetch해서 역직렬화한다.
async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, FetchError> {
    let resp = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| FetchError::Other(e.to_string()))?;
    if resp.status() == 404 {
        return Err(FetchError::NotFound);
    }
    if !resp.ok() {
        return Err(FetchError::Status(resp.status()));
    }
    // dx serve 개발 서버는 없는 파일에도 404 대신 SPA 폴백(index.html)을
    // 200으로 돌려준다. Content-Type이 JSON이 아니면 "데이터 없음"으로 간주해
    // 정적 배포(진짜 404)와 동일한 안내를 보여준다.
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.contains("json") {
        return Err(FetchError::NotFound);
    }
    resp.json::<T>()
        .await
        .map_err(|e| FetchError::Other(e.to_string()))
}

/// 한자 상세 JSON을 불러온다. URL의 한자는 브라우저 fetch가 자동으로
/// percent-encoding 처리한다.
pub async fn fetch_kanji(character: &str) -> Result<KanjiDetail, FetchError> {
    fetch_json(&format!("{DATA_DIR}/kanji/{character}.json")).await
}

/// 전체 한자 요약 목록(`kanji-list.json`)을 불러온다.
pub async fn fetch_kanji_list() -> Result<Vec<KanjiSummary>, FetchError> {
    fetch_json(&format!("{DATA_DIR}/kanji-list.json")).await
}

/// 부수 상세 JSON(`radicals/{부수}.json`)을 불러온다.
pub async fn fetch_radical(radical: &str) -> Result<RadicalDetail, FetchError> {
    fetch_json(&format!("{DATA_DIR}/radicals/{radical}.json")).await
}

/// 전체 부수 요약 목록(`radicals-list.json`)을 불러온다 (/radicals 인덱스).
pub async fn fetch_radicals_list() -> Result<Vec<RadicalSummary>, FetchError> {
    fetch_json(&format!("{DATA_DIR}/radicals-list.json")).await
}

/// 부수/부품 → 그 부품을 가진 한자 목록 역인덱스(`by-radical.json`)를 불러온다.
pub async fn fetch_by_radical() -> Result<HashMap<String, Vec<String>>, FetchError> {
    fetch_json(&format!("{DATA_DIR}/by-radical.json")).await
}

/// 검색 인덱스(`search-index.json`)를 불러온다 (M5 — 검색 모달 첫 오픈 시 lazy).
pub async fn fetch_search_index() -> Result<crate::search::SearchIndex, FetchError> {
    fetch_json(&format!("{DATA_DIR}/search-index.json")).await
}

/// 어원 서술 마크다운 → HTML 변환 (pulldown-cmark, wasm 호환).
pub fn markdown_to_html(markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, options);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

/// "오늘의 한자" 인덱스 — UTC 일수 기반 결정적 선택.
/// 같은 날에는 항상 같은 한자가 나온다.
pub fn today_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    // js Date.now()는 UTC epoch 밀리초. 일 단위로 자른다.
    let days = (js_sys::Date::now() / 86_400_000.0).floor() as u64;
    (days % len as u64) as usize
}

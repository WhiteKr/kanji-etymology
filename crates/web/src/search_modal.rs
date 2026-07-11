//! 검색 모달 + 검색 데이터 전역 캐시 (M5).
//!
//! - 모달 열림 상태는 전역 시그널 — 헤더·랜딩 어디서든 열 수 있다.
//! - 인덱스(search-index.json)와 요약(kanji-list.json)은 모달 첫 오픈 때
//!   lazy fetch하고 메모리에 캐시한다 (설계 문서 9장).
//! - 이후 입력은 전부 인메모리 매칭이라 디바운스 없이 즉시 갱신한다.

use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::api::{self, FetchError, KanjiSummary};
use crate::search::{self, SearchIndex};
use crate::Route;

/// 검색 모달 열림 상태.
static SEARCH_OPEN: GlobalSignal<bool> = GlobalSignal::new(|| false);

/// 검색 데이터 메모리 캐시 — 한 번 채워지면 재fetch하지 않는다.
static SEARCH_DATA: GlobalSignal<Option<Rc<SearchData>>> = GlobalSignal::new(|| None);

/// 모달에 한 번에 보여줄 최대 결과 수 (전체는 결과 페이지에서).
const MODAL_MAX_RESULTS: usize = 12;

/// 인덱스 + 결과 카드 표시용 요약 정보 묶음.
pub struct SearchData {
    pub index: SearchIndex,
    /// 한자 → 요약 (한국음·뜻·JLPT 표시용)
    summaries: HashMap<String, KanjiSummary>,
}

impl SearchData {
    pub fn summary_of(&self, kanji: &str) -> Option<&KanjiSummary> {
        self.summaries.get(kanji)
    }
}

/// 검색 모달을 연다 (헤더 버튼·랜딩 검색 바·`/` 단축키 공용).
pub fn open_search() {
    *SEARCH_OPEN.write() = true;
}

fn close_search() {
    *SEARCH_OPEN.write() = false;
}

/// 검색 데이터를 캐시에서 꺼내거나, 없으면 fetch해서 채운 뒤 돌려준다.
pub async fn load_search_data() -> Result<Rc<SearchData>, FetchError> {
    // 읽기 borrow는 이 문장에서 끝난다 (await 건너 borrow 유지 금지)
    let cached = SEARCH_DATA.read().clone();
    if let Some(data) = cached {
        return Ok(data);
    }
    let index = api::fetch_search_index().await?;
    let list = api::fetch_kanji_list().await?;
    let summaries = list.into_iter().map(|s| (s.character.clone(), s)).collect();
    let data = Rc::new(SearchData { index, summaries });
    *SEARCH_DATA.write() = Some(data.clone());
    Ok(data)
}

/// 모달 래퍼 — 닫혀 있으면 아무것도 렌더하지 않는다.
/// 열릴 때마다 내부 컴포넌트가 새로 마운트되어 입력이 초기화·포커스된다.
#[component]
pub fn SearchModal() -> Element {
    if !SEARCH_OPEN() {
        return rsx! {};
    }
    rsx! {
        SearchModalInner {}
    }
}

#[component]
fn SearchModalInner() -> Element {
    // 첫 오픈 시 lazy fetch — 이후엔 캐시 히트라 즉시 완료된다.
    let data = use_resource(load_search_data);
    let mut query = use_signal(String::new);
    let nav = navigator();

    // Enter → 전체 결과 페이지로 이동
    let go_to_results = move || {
        let q = query().trim().to_string();
        if !q.is_empty() {
            close_search();
            nav.push(Route::SearchPage { q });
        }
    };

    rsx! {
        // 바깥(오버레이) 클릭으로 닫기 — 패널 클릭은 전파를 끊는다
        div {
            class: "search-modal__overlay",
            onclick: move |_| close_search(),
            div {
                class: "search-modal",
                role: "dialog",
                aria_label: "검색",
                onclick: move |e| e.stop_propagation(),

                input {
                    class: "search-modal__input",
                    r#type: "search",
                    placeholder: "한자 · 한국음 · 뜻 · 가나 · 로마자",
                    aria_label: "검색어",
                    value: "{query}",
                    // 동적으로 삽입되는 요소라 autofocus 속성 대신 직접 포커스
                    onmounted: move |e| async move {
                        let _ = e.set_focus(true).await;
                    },
                    oninput: move |e| query.set(e.value()),
                    onkeydown: move |e| match e.key() {
                        Key::Escape => close_search(),
                        Key::Enter => go_to_results(),
                        _ => {}
                    },
                }

                match &*data.read() {
                    None => rsx! {
                        p { class: "search-modal__status", "검색 인덱스를 불러오는 중…" }
                    },
                    Some(Err(err)) => rsx! {
                        p { class: "search-modal__status", "검색 인덱스를 불러오지 못했습니다: {err}" }
                    },
                    Some(Ok(data)) => modal_results(data, &query()),
                }
            }
        }
    }
}

/// 모달 내부 결과 목록 (입력할 때마다 인메모리 재계산).
fn modal_results(data: &Rc<SearchData>, query: &str) -> Element {
    let q = query.trim();
    if q.is_empty() {
        return rsx! {
            p { class: "search-modal__status",
                "한자(学) · 한국음(학) · 뜻(배우다) · 가나(まなぶ) · 로마자(manabu) 무엇으로든 찾아보세요."
            }
        };
    }

    let hits = search::search(&data.index, q);
    if hits.is_empty() {
        return rsx! {
            p { class: "search-modal__status", "“{q}”에 해당하는 한자가 없습니다." }
        };
    }

    let total = hits.len();
    let q_owned = q.to_string();

    rsx! {
        ul { class: "search-modal__results",
            for hit in hits.into_iter().take(MODAL_MAX_RESULTS) {
                li {
                    Link {
                        class: "search-result",
                        to: Route::KanjiPage { character: hit.kanji.clone() },
                        onclick: move |_| close_search(),
                        span { class: "hanja search-result__char", "{hit.kanji}" }
                        if let Some(summary) = data.summary_of(&hit.kanji) {
                            span { class: "search-result__reading", "{summary.korean_reading}" }
                            span { class: "search-result__meaning",
                                {summary.meanings.join(" · ")}
                            }
                            if let Some(jlpt) = summary.jlpt.as_deref() {
                                span { class: "search-result__jlpt", "{jlpt}" }
                            }
                        }
                    }
                }
            }
        }
        Link {
            class: "search-modal__all",
            to: Route::SearchPage { q: q_owned },
            onclick: move |_| close_search(),
            "전체 결과 {total}건 보기 (Enter)"
        }
    }
}

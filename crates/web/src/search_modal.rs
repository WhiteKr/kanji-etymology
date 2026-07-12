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

/// 모달 자동완성에 한 번에 보여줄 최대 후보 수 (전체는 결과 페이지에서).
const MODAL_TYPEAHEAD_MAX: usize = 8;

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
    // 자동완성 하이라이트 인덱스 — None이면 하이라이트 없음(Enter는 결과 페이지로).
    let mut active_index = use_signal(|| None::<usize>);
    let nav = navigator();

    // 현재 입력에 대한 자동완성 후보. 키보드 탐색(↓/↑/Enter)과 목록 렌더링이
    // 정확히 같은 배열을 봐야 하므로 렌더마다 한 번만 계산해서 공유한다.
    // 랭킹은 search::top_matches → search::search를 그대로 재사용.
    let candidates: Vec<search::SearchHit> = match &*data.read() {
        Some(Ok(d)) if !query().trim().is_empty() => {
            search::top_matches(&d.index, query().trim(), MODAL_TYPEAHEAD_MAX)
        }
        _ => Vec::new(),
    };
    let candidate_count = candidates.len();

    // Enter(하이라이트 없음) → 전체 결과 페이지로 이동
    let go_to_results = move || {
        let q = query().trim().to_string();
        if !q.is_empty() {
            close_search();
            nav.push(Route::SearchPage { q });
        }
    };

    // Enter → 하이라이트된 후보가 있으면 그 한자 페이지로 바로 이동, 없으면 결과 페이지로.
    let candidates_for_enter = candidates.clone();
    let on_enter = move || {
        if let Some(hit) = active_index().and_then(|idx| candidates_for_enter.get(idx)) {
            let kanji = hit.kanji.clone();
            close_search();
            nav.push(Route::KanjiPage { character: kanji });
        } else {
            go_to_results();
        }
    };

    // ↓/↑ — 후보가 없으면 무시, 있으면 순환(마지막→처음, 처음→마지막) 이동.
    let mut move_active = move |delta: isize| {
        if candidate_count == 0 {
            return;
        }
        let len = candidate_count as isize;
        let next = match active_index() {
            None if delta >= 0 => 0,
            None => len - 1,
            Some(idx) => (idx as isize + delta).rem_euclid(len),
        };
        active_index.set(Some(next as usize));
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
                    oninput: move |e| {
                        query.set(e.value());
                        // 입력이 바뀌면 후보 목록도 바뀌므로 하이라이트는 초기화.
                        active_index.set(None);
                    },
                    onkeydown: move |e| match e.key() {
                        Key::Escape => close_search(),
                        Key::Enter => on_enter(),
                        Key::ArrowDown => {
                            e.prevent_default();
                            move_active(1);
                        }
                        Key::ArrowUp => {
                            e.prevent_default();
                            move_active(-1);
                        }
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
                    Some(Ok(data)) => modal_results(data, &query(), &candidates, active_index),
                }
            }
        }
    }
}

/// 모달 내부 자동완성 목록 (입력할 때마다 인메모리 재계산).
/// `candidates`는 이미 `search::top_matches`로 잘라낸 상위 목록이고,
/// `active_index`는 키보드/마우스로 하이라이트된 후보의 인덱스다.
fn modal_results(
    data: &Rc<SearchData>,
    query: &str,
    candidates: &[search::SearchHit],
    mut active_index: Signal<Option<usize>>,
) -> Element {
    let q = query.trim();
    if q.is_empty() {
        return rsx! {
            p { class: "search-modal__status",
                "한자(学) · 한국음(학) · 뜻(배우다) · 가나(まなぶ) · 로마자(manabu) 무엇으로든 찾아보세요."
            }
        };
    }

    if candidates.is_empty() {
        return rsx! {
            p { class: "search-modal__status", "“{q}”에 해당하는 한자가 없습니다." }
        };
    }

    // 전체 결과 건수는 자동완성 후보와 별개로 (잘리지 않은) 전체 검색으로 센다.
    let total = search::search(&data.index, q).len();
    let q_owned = q.to_string();
    let active = active_index();

    rsx! {
        ul {
            class: "search-modal__results",
            role: "listbox",
            aria_label: "검색 후보",
            for (idx , hit) in candidates.iter().cloned().enumerate() {
                li {
                    key: "{hit.kanji}",
                    role: "option",
                    aria_selected: if active == Some(idx) { "true" } else { "false" },
                    onmouseenter: move |_| active_index.set(Some(idx)),
                    Link {
                        class: if active == Some(idx) {
                            "search-result search-result--active"
                        } else {
                            "search-result"
                        },
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

//! `/search?q=...` 검색 결과 페이지 (M5).
//!
//! 검색 모달과 같은 전역 캐시(search_modal::load_search_data)를 쓰므로
//! 모달을 거쳐 왔다면 재fetch 없이 즉시 렌더된다.

use dioxus::prelude::*;

use crate::search;
use crate::search_modal::{self, SearchData};
use crate::Route;

#[component]
pub fn SearchPage(q: ReadSignal<String>) -> Element {
    // q는 라우트 쿼리 파라미터 — 같은 라우트에서 q만 바뀌어도 재렌더된다.
    // 데이터 자체는 전역 캐시라 resource는 사실상 최초 1회만 fetch한다.
    let data = use_resource(search_modal::load_search_data);

    rsx! {
        document::Title { "검색: {q} — 한자 어원 사전" }
        main { class: "page search-page",
            h1 { class: "search-page__title", "검색 결과" }

            if q().trim().is_empty() {
                section { class: "status-block",
                    p { "검색어가 없습니다." }
                    button {
                        class: "button-link search-page__open",
                        r#type: "button",
                        onclick: move |_| search_modal::open_search(),
                        "검색 열기"
                    }
                }
            } else {
                match &*data.read() {
                    None => rsx! {
                        p { class: "status-message", "검색 인덱스를 불러오는 중…" }
                    },
                    Some(Err(err)) => rsx! {
                        section { class: "status-block",
                            h1 { "검색 인덱스를 불러오지 못했습니다" }
                            p { "{err}" }
                        }
                    },
                    Some(Ok(data)) => result_grid(data, q().trim()),
                }
            }
        }
    }
}

/// 결과 카드 그리드 — kanji-list.json 요약(한자·한국음·뜻·JLPT)으로 채운다.
fn result_grid(data: &std::rc::Rc<SearchData>, q: &str) -> Element {
    let hits = search::search(&data.index, q);

    if hits.is_empty() {
        return rsx! {
            section { class: "status-block",
                h1 { "“{q}”에 해당하는 한자가 없습니다" }
                p { "한자·한국 한자음·한국어 뜻·일본어 읽기(가나/로마자)로 다시 검색해 보세요." }
                p { "찾는 한자가 아직 등재되지 않았을 수도 있어요. 콘텐츠는 계속 추가되고 있습니다." }
            }
        };
    }

    let count = hits.len();

    rsx! {
        p { class: "search-page__summary", "“{q}” — {count}건" }
        div { class: "kanji-grid",
            for hit in hits {
                Link {
                    class: "kanji-card",
                    to: Route::KanjiPage { character: hit.kanji.clone() },
                    span { class: "hanja kanji-card__char", "{hit.kanji}" }
                    if let Some(summary) = data.summary_of(&hit.kanji) {
                        span { class: "kanji-card__reading", "{summary.korean_reading}" }
                        if let Some(first) = summary.meanings.first() {
                            span { class: "kanji-card__meaning", "{first}" }
                        }
                        if let Some(jlpt) = summary.jlpt.as_deref() {
                            span { class: "kanji-card__jlpt", "{jlpt}" }
                        }
                    }
                }
            }
        }
    }
}

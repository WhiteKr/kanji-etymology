//! catch-all 404 페이지 — 친절한 안내 + 등재 한자 추천 + 검색 진입 (M6).

use dioxus::prelude::*;

use crate::api;
use crate::search_modal;
use crate::Route;

/// 추천으로 보여줄 최대 한자 수.
const SUGGESTION_COUNT: usize = 6;

#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = format!("/{}", segments.join("/"));

    rsx! {
        document::Title { "페이지를 찾을 수 없습니다 — 한자 어원 사전" }
        main { class: "page",
            section { class: "status-block",
                h1 { "페이지를 찾을 수 없습니다" }
                p {
                    code { class: "status-block__path", "{path}" }
                    " 주소에 해당하는 페이지가 없어요. 주소가 바뀌었거나, 아직 준비되지 않은 콘텐츠일 수 있습니다."
                }
                Link { class: "button-link", to: Route::Landing {}, "홈으로 돌아가기" }
            }

            KanjiSuggestions {}
        }
    }
}

/// "대신 이런 한자는 어때요" 추천 블록 — 404·미등재 한자 페이지 공용.
/// kanji-list에서 오늘의 한자 인덱스를 기점으로 몇 자를 돌아가며 추천하고,
/// 검색 모달을 여는 버튼을 함께 보여준다.
#[component]
pub fn KanjiSuggestions() -> Element {
    let list = use_resource(|| async { api::fetch_kanji_list().await });

    // 목록을 불러오지 못해도 검색 진입은 항상 가능해야 한다.
    let picks = match &*list.read() {
        Some(Ok(list)) if !list.is_empty() => {
            let start = api::today_index(list.len());
            (0..list.len().min(SUGGESTION_COUNT))
                .map(|i| list[(start + i) % list.len()].clone())
                .collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };

    rsx! {
        section { class: "section suggest",
            if !picks.is_empty() {
                h2 { class: "section__title", "대신 이런 한자는 어때요?" }
                div { class: "kanji-grid",
                    for item in picks.iter() {
                        Link {
                            class: "kanji-card",
                            to: Route::KanjiPage { character: item.character.clone() },
                            span { class: "hanja kanji-card__char", "{item.character}" }
                            span { class: "kanji-card__reading", "{item.korean_reading}" }
                            if let Some(first) = item.meanings.first() {
                                span { class: "kanji-card__meaning", "{first}" }
                            }
                        }
                    }
                }
            }
            div { class: "suggest__search",
                button {
                    class: "button-link suggest__search-button",
                    r#type: "button",
                    onclick: move |_| search_modal::open_search(),
                    "검색으로 찾아보기"
                }
            }
        }
    }
}

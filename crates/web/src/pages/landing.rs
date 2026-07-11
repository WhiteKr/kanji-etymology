//! 랜딩 페이지 — 검색 바(M5 모달 오픈), 오늘의 한자, 전체 한자 카드 그리드.

use dioxus::prelude::*;

use crate::api::{self, KanjiSummary};
use crate::search_modal;
use crate::Route;

/// 한자 요약 → 한자 페이지 라우트.
fn kanji_route(character: &str) -> Route {
    Route::KanjiPage {
        character: character.to_string(),
    }
}

#[component]
pub fn Landing() -> Element {
    let list = use_resource(|| async { api::fetch_kanji_list().await });

    rsx! {
        document::Title { "한자 어원 사전" }
        main { class: "page landing",
            header { class: "landing__hero",
                h1 { class: "landing__title", "한자 어원 사전" }
                p { class: "landing__tagline", "한자를 외우지 않고 이해하기 — 어원 스토리와 부수 분해" }

                // 검색 바 — 클릭하면 검색 모달이 열린다 (M5).
                // 실제 입력은 모달에서 받으므로 input이 아닌 버튼으로 렌더.
                button {
                    class: "search-bar",
                    r#type: "button",
                    aria_label: "검색 열기",
                    onclick: move |_| search_modal::open_search(),
                    span { class: "search-bar__hint", "한자·한국음·뜻·일본어로 검색" }
                    kbd { class: "search-bar__kbd", "/" }
                }
            }

            match &*list.read() {
                None => rsx! {
                    p { class: "status-message", "불러오는 중…" }
                },
                Some(Err(err)) => rsx! {
                    section { class: "status-block",
                        h1 { "한자 목록을 불러오지 못했습니다" }
                        p { "{err}" }
                    }
                },
                Some(Ok(list)) => rsx! {
                    TodayKanji { list: list.clone() }
                    KanjiGrid { list: list.clone() }
                },
            }
        }
    }
}

/// 오늘의 한자 — UTC 날짜 기반으로 결정적으로 하나를 고른다.
#[component]
fn TodayKanji(list: Vec<KanjiSummary>) -> Element {
    let Some(today) = list.get(api::today_index(list.len())) else {
        return rsx! {};
    };
    let meanings = today.meanings.join(" · ");

    rsx! {
        section { class: "section today",
            h2 { class: "section__title", "오늘의 한자" }
            Link { class: "today__card", to: kanji_route(&today.character),
                span { class: "hanja today__char", "{today.character}" }
                span { class: "today__info",
                    span { class: "today__reading", "{today.korean_reading}" }
                    span { class: "today__meanings", "{meanings}" }
                }
            }
        }
    }
}

/// 전체 한자 카드 그리드 (kanji-list.json 기반).
#[component]
fn KanjiGrid(list: Vec<KanjiSummary>) -> Element {
    rsx! {
        section { class: "section browse",
            h2 { class: "section__title", "전체 한자" }
            div { class: "kanji-grid",
                for item in list.iter() {
                    Link { class: "kanji-card", to: kanji_route(&item.character),
                        span { class: "hanja kanji-card__char", "{item.character}" }
                        span { class: "kanji-card__reading", "{item.korean_reading}" }
                        if let Some(first) = item.meanings.first() {
                            span { class: "kanji-card__meaning", "{first}" }
                        }
                        if let Some(jlpt) = item.jlpt.as_deref() {
                            span { class: "kanji-card__jlpt", "{jlpt}" }
                        }
                    }
                }
            }
        }
    }
}

//! `/radicals` 부수 인덱스 (M6) — 전체 부수 카드 목록.
//! build-data가 생성하는 radicals-list.json(부수 요약 + 소속 한자 수)을 쓴다.

use dioxus::prelude::*;

use crate::api;
use crate::Route;

#[component]
pub fn RadicalsPage() -> Element {
    let list = use_resource(|| async { api::fetch_radicals_list().await });

    rsx! {
        document::Title { "부수 일람 — 한자 어원 사전" }
        main { class: "page radicals-page",
            h1 { class: "page-title", "부수 일람" }
            p { class: "page-lead", "부수를 알면 처음 보는 한자도 뜻을 짐작할 수 있어요. 어원이 등재된 부수 목록입니다." }

            match &*list.read() {
                None => rsx! {
                    p { class: "status-message", "불러오는 중…" }
                },
                Some(Err(err)) => rsx! {
                    section { class: "status-block",
                        h1 { "부수 목록을 불러오지 못했습니다" }
                        p { "{err}" }
                    }
                },
                Some(Ok(list)) if list.is_empty() => rsx! {
                    section { class: "status-block",
                        h1 { "등재된 부수가 아직 없습니다" }
                        p { "부수 콘텐츠는 계속 추가되고 있습니다." }
                        Link { class: "button-link", to: Route::Landing {}, "홈으로 돌아가기" }
                    }
                },
                Some(Ok(list)) => rsx! {
                    section { class: "section",
                        div { class: "radical-list",
                            for item in list.iter() {
                                Link {
                                    class: "radical-card",
                                    to: Route::RadicalPage { radical: item.radical.clone() },
                                    span { class: "hanja radical-card__char", "{item.radical}" }
                                    span { class: "radical-card__info",
                                        span { class: "radical-card__name", "{item.name}" }
                                        span { class: "radical-card__meaning", "{item.meaning}" }
                                    }
                                    span { class: "radical-card__meta",
                                        span { class: "radical-card__strokes", "{item.stroke_count}획" }
                                        span { class: "radical-card__count", "한자 {item.kanji_count}자" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

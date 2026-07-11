//! 부수 페이지 (M6) — 부수 헤더(문자·이름·의미·획수) + 어원 서술 +
//! "이 부수를 가진 한자" 그리드 (by-radical 역인덱스 + kanji-list 요약).

use dioxus::prelude::*;

use crate::api::{self, FetchError, KanjiSummary, RadicalDetail};
use crate::Route;

#[component]
pub fn RadicalPage(radical: ReadSignal<String>) -> Element {
    // radical 시그널을 구독하므로 같은 라우트에서 부수만 바뀌어도 다시 fetch한다.
    let detail = use_resource(move || async move { api::fetch_radical(&radical()).await });

    // 소속 한자 그리드용 데이터. 부수 상세가 미등재(404)여도 역인덱스에
    // 부품으로는 존재할 수 있으므로 상세와 독립적으로 불러온다.
    let members = use_resource(move || async move {
        let by_radical = api::fetch_by_radical().await?;
        let list = api::fetch_kanji_list().await?;
        let chars = by_radical.get(&radical()).cloned().unwrap_or_default();
        let summaries: Vec<KanjiSummary> = chars
            .iter()
            .filter_map(|c| list.iter().find(|s| &s.character == c).cloned())
            .collect();
        Ok::<Vec<KanjiSummary>, FetchError>(summaries)
    });

    rsx! {
        document::Title { "{radical} 부수 — 한자 어원 사전" }
        main { class: "page radical-page",
            match &*detail.read() {
                None => rsx! {
                    p { class: "status-message", "불러오는 중…" }
                },
                Some(Err(FetchError::NotFound)) => rsx! {
                    section { class: "status-block",
                        h1 { "아직 등재되지 않은 부수입니다" }
                        p {
                            span { class: "hanja status-block__char", "{radical}" }
                            " 부수의 어원 설명은 준비 중이에요. 콘텐츠는 계속 추가되고 있습니다."
                        }
                        Link { class: "button-link", to: Route::RadicalsPage {}, "부수 일람 보기" }
                    }
                },
                Some(Err(err)) => rsx! {
                    section { class: "status-block",
                        h1 { "불러오지 못했습니다" }
                        p { "{err}" }
                        Link { class: "button-link", to: Route::Landing {}, "홈으로 돌아가기" }
                    }
                },
                Some(Ok(detail)) => rsx! {
                    RadicalArticle { detail: detail.clone() }
                },
            }

            // 이 부수를 가진 한자 — 상세 등재 여부와 무관하게 표시한다.
            match &*members.read() {
                Some(Ok(list)) if !list.is_empty() => rsx! {
                    MemberGrid { list: list.clone() }
                },
                _ => rsx! {},
            }
        }
    }
}

/// 부수 상세 로딩 성공 시의 본문 — 헤더 + 어원 서술.
#[component]
fn RadicalArticle(detail: RadicalDetail) -> Element {
    let entry = &detail.entry;
    let body_html = api::markdown_to_html(&detail.body_markdown);
    let variants = entry.variants.join(" · ");

    rsx! {
        article {
            header { class: "radical-header",
                h1 { class: "hanja radical-header__char", "{entry.radical}" }
                div { class: "radical-header__badges",
                    span { class: "badge badge--korean", "{entry.name}" }
                    span { class: "badge badge--kun", "{entry.stroke_count}획" }
                    if !variants.is_empty() {
                        span { class: "badge badge--on hanja", "이체자 {variants}" }
                    }
                }
                p { class: "radical-header__meaning", "{entry.meaning}" }
            }

            section { class: "section etymology",
                h2 { class: "section__title", "왜 이 모양이 되었나" }
                // pulldown-cmark로 변환한 HTML. 콘텐츠는 저장소 내부에서
                // 검수를 거치므로 dangerous_inner_html 사용이 허용된다.
                div { class: "etymology__body", dangerous_inner_html: "{body_html}" }
            }
        }
    }
}

/// "이 부수를 가진 한자" 카드 그리드.
#[component]
fn MemberGrid(list: Vec<KanjiSummary>) -> Element {
    rsx! {
        section { class: "section radical-members",
            h2 { class: "section__title", "이 부수를 가진 한자" }
            div { class: "kanji-grid",
                for item in list.iter() {
                    Link {
                        class: "kanji-card",
                        to: Route::KanjiPage { character: item.character.clone() },
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

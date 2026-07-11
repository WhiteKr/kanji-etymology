//! 한자 페이지 — 설계 문서 5장(UX) 레이아웃.
//!
//! 위에서 아래로: 신뢰도 배지 → 헤더(한자·읽기·뜻) → 자형의 변천 →
//! 구성(부품) → 왜 이 모양이 되었나(어원 서술) → 더 깊이(단어·관련·출처·정정 제안).

use dioxus::prelude::*;
use kanji_schema::{Component, Confidence, Evolution, Related, Source, Word};

use crate::api::{self, FetchError, KanjiDetail};
use crate::Route;

/// 신뢰도 값 → (표시 문구, CSS modifier 클래스).
fn confidence_label(confidence: Confidence) -> (&'static str, &'static str) {
    match confidence {
        Confidence::WellAttested => ("근거 확실", "confidence--well"),
        Confidence::Interpretation => ("해석", "confidence--interp"),
        Confidence::Tentative => ("잠정", "confidence--tentative"),
    }
}

/// 신뢰도 배지에 붙는 부연 설명.
fn confidence_note(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::WellAttested => "정설로 널리 받아들여지는 설명입니다.",
        Confidence::Interpretation => "여러 학설 중 하나의 해석입니다.",
        Confidence::Tentative => "근거가 약한 잠정적 추정입니다.",
    }
}

/// 콘텐츠의 내부 링크 문자열(`/kanji/学`, `/radical/冖`)을 라우트로 변환.
/// 형식이 다르면 `None` (링크 없이 렌더).
fn route_for_link(link: &str) -> Option<Route> {
    if let Some(target) = link.strip_prefix("/kanji/") {
        Some(Route::KanjiPage {
            character: target.to_string(),
        })
    } else {
        link.strip_prefix("/radical/").map(|target| Route::RadicalPage {
            radical: target.to_string(),
        })
    }
}

#[component]
pub fn KanjiPage(character: ReadSignal<String>) -> Element {
    // character 시그널을 구독하므로 같은 라우트 안에서 한자만 바뀌어도 다시 fetch한다.
    let detail = use_resource(move || async move { api::fetch_kanji(&character()).await });

    rsx! {
        document::Title { "{character} — 한자 어원 사전" }
        main { class: "page kanji-page",
            match &*detail.read() {
                None => rsx! {
                    p { class: "status-message", "불러오는 중…" }
                },
                Some(Err(FetchError::NotFound)) => rsx! {
                    section { class: "status-block",
                        h1 { "아직 등재되지 않은 한자입니다" }
                        p {
                            span { class: "hanja status-block__char", "{character}" }
                            " 페이지는 준비 중이에요. 콘텐츠는 계속 추가되고 있습니다."
                        }
                        Link { class: "button-link", to: Route::Landing {}, "홈으로 돌아가기" }
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
                    KanjiArticle { detail: detail.clone() }
                },
            }
        }
    }
}

/// 데이터 로딩 성공 시의 본문 전체.
#[component]
fn KanjiArticle(detail: KanjiDetail) -> Element {
    let entry = &detail.entry;
    let (conf_label, conf_class) = confidence_label(entry.confidence);
    let body_html = api::markdown_to_html(&detail.body_markdown);
    let meanings = entry.meanings.join(" · ");

    rsx! {
        article {
            // ── 신뢰도 배지 (페이지 상단) ─────────────────────────
            div { class: "confidence {conf_class}",
                span { class: "confidence__label", "{conf_label}" }
                span { class: "confidence__note", {confidence_note(entry.confidence)} }
            }

            // ── 헤더: 한자 크게 + 읽기 배지 + 뜻 ─────────────────
            header { class: "kanji-header",
                h1 { class: "hanja kanji-header__char", "{entry.character}" }
                div { class: "kanji-header__badges",
                    span { class: "badge badge--korean", "{entry.korean.reading}" }
                    for on in entry.readings.on.iter() {
                        span { class: "badge badge--on", "音 " span { class: "hanja", "{on}" } }
                    }
                    for kun in entry.readings.kun.iter() {
                        span { class: "badge badge--kun", "訓 " span { class: "hanja", "{kun.reading}" } }
                    }
                }
                p { class: "kanji-header__meanings", "{meanings}" }
            }

            // ── 자형의 변천 (없으면 섹션 생략) ────────────────────
            if !entry.evolution.is_empty() {
                EvolutionStrip { steps: entry.evolution.clone() }
            }

            // ── 구성 (부품 분해) ─────────────────────────────────
            if !entry.components.is_empty() {
                ComponentCards { components: entry.components.clone() }
            }

            // ── 왜 이 모양이 되었나 (어원 서술) ───────────────────
            section { class: "section etymology",
                h2 { class: "section__title", "왜 이 모양이 되었나" }
                // pulldown-cmark로 변환한 HTML. 콘텐츠는 저장소 내부에서
                // 검수를 거치므로 dangerous_inner_html 사용이 허용된다.
                div { class: "etymology__body", dangerous_inner_html: "{body_html}" }
            }

            // ── 더 깊이 ──────────────────────────────────────────
            DeeperSection {
                words: entry.words.clone(),
                related: entry.related.clone(),
                sources: entry.sources.clone(),
            }
        }
    }
}

/// 자형 변천 가로 스트립.
#[component]
fn EvolutionStrip(steps: Vec<Evolution>) -> Element {
    rsx! {
        section { class: "section evolution",
            h2 { class: "section__title", "자형의 변천" }
            div { class: "evolution__strip",
                for (i, step) in steps.iter().enumerate() {
                    if i > 0 {
                        span { class: "evolution__arrow", aria_hidden: "true", "→" }
                    }
                    div { class: "evolution__step",
                        span { class: "hanja evolution__glyph", "{step.form}" }
                        span { class: "evolution__era", "{step.era}" }
                    }
                }
            }
        }
    }
}

/// 부품 카드 목록. link가 있으면 해당 라우트로 이동한다.
#[component]
fn ComponentCards(components: Vec<Component>) -> Element {
    rsx! {
        section { class: "section components",
            h2 { class: "section__title", "구성 — 클릭하여 부품으로 이동" }
            div { class: "components__grid",
                for comp in components.iter() {
                    {
                        let card_body = rsx! {
                            span { class: "hanja component-card__form", "{comp.form}" }
                            span { class: "component-card__role", "{comp.role}" }
                        };
                        match comp.link.as_deref().and_then(route_for_link) {
                            Some(route) => rsx! {
                                Link { class: "component-card component-card--link", to: route,
                                    {card_body}
                                }
                            },
                            None => rsx! {
                                div { class: "component-card", {card_body} }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// "더 깊이" 섹션 — 단어, 관련 한자, 출처, 정정 제안.
#[component]
fn DeeperSection(words: Vec<Word>, related: Vec<Related>, sources: Vec<Source>) -> Element {
    rsx! {
        section { class: "section deeper",
            h2 { class: "section__title", "더 깊이" }

            if !words.is_empty() {
                h3 { class: "deeper__subtitle", "이 한자가 들어간 단어" }
                ul { class: "word-list",
                    for word in words.iter() {
                        li { class: "word-list__item",
                            span { class: "hanja word-list__word", "{word.word}" }
                            span { class: "hanja word-list__reading", "{word.reading}" }
                            span { class: "word-list__gloss", "{word.gloss}" }
                        }
                    }
                }
            }

            if !related.is_empty() {
                h3 { class: "deeper__subtitle", "관련 한자" }
                ul { class: "related-list",
                    for rel in related.iter() {
                        li { class: "related-list__item",
                            Link {
                                class: "related-list__link",
                                to: Route::KanjiPage { character: rel.kanji.clone() },
                                span { class: "hanja related-list__char", "{rel.kanji}" }
                            }
                            span { class: "related-list__relation", "{rel.relation}" }
                        }
                    }
                }
            }

            if !sources.is_empty() {
                h3 { class: "deeper__subtitle", "출처" }
                ul { class: "source-list",
                    for source in sources.iter() {
                        li { class: "source-list__item",
                            if let Some(url) = source.url.as_deref() {
                                a { class: "source-list__name", href: "{url}",
                                    target: "_blank", rel: "noopener noreferrer",
                                    "{source.name}"
                                }
                            } else {
                                span { class: "source-list__name", "{source.name}" }
                            }
                            if let Some(r) = source.r#ref.as_deref() {
                                span { class: "source-list__meta", " {r}" }
                            }
                            if let Some(year) = source.year {
                                span { class: "source-list__meta", " ({year})" }
                            }
                            if let Some(license) = source.license.as_deref() {
                                span { class: "source-list__meta", " · {license}" }
                            }
                        }
                    }
                }
            }

            // 정정 제안 — 자리만. 실제 제출 흐름(모달 → Worker → GitHub Issue)은 M7.
            div { class: "feedback",
                button { class: "feedback__button", r#type: "button",
                    title: "정정 제안 기능은 준비 중입니다 (M7)",
                    "⚠️ 이 어원 설명에 이의 제기 / 정정 제안"
                }
                p { class: "feedback__note", "정정 제안 기능은 준비 중입니다." }
            }
        }
    }
}

//! `/about` 소개 페이지 (M6) — 비전·방법론·신뢰도 라벨·한계·기여 안내.
//! 내용은 설계 문서(docs/2026-05-27-kanji-etymology-mvp-design.md)
//! 1장(비전)·5장(신뢰도)·8장(데이터 소스 위계)·여정 C(정정 제안)에서 발췌·요약했다.

use dioxus::prelude::*;

use crate::Route;

/// 8장 데이터 소스 위계 표의 한 줄: (데이터 종류, 1차 출처, 라이선스, 신뢰도).
const DATA_SOURCES: [(&str, &str, &str, &str); 7] = [
    ("자형 (글자)", "Unicode + KANJIDIC2", "—", "절대 신뢰"),
    ("읽기 (음·훈)", "KANJIDIC2", "CC BY-SA", "절대 신뢰"),
    ("획수·부수·JLPT", "KANJIDIC2", "CC BY-SA", "절대 신뢰"),
    ("부품 분해", "KRADFILE + 검수", "CC BY-SA", "거의 신뢰"),
    ("자형 변천 글리프", "GlyphWiki / 說文解字 archive", "CC BY-SA", "출처 명시"),
    ("어원 서술", "說文解字 + Wiktionary + 자체 정리", "혼합", "\"해석\"으로 표시"),
    ("한국 한자음·뜻", "표준국어대사전·한국한자어사전 등", "공공", "절대 신뢰"),
];

#[component]
pub fn AboutPage() -> Element {
    rsx! {
        document::Title { "소개 — 한자 어원 사전" }
        main { class: "page about-page",
            h1 { class: "page-title", "이 사전에 대하여" }

            // ── 비전 (설계 문서 1장) ─────────────────────────────
            section { class: "section",
                h2 { class: "section__title", "무엇을 하려는 사전인가" }
                blockquote { class: "about__quote",
                    "한자를 외우지 않고 이해하는 한국인 학습자를 위한, 어원 스토리와 부수 분해 중심의 한자 사전."
                }
                p {
                    "암기 위주 학습 대신, 자형의 역사·부품 분해·서술형 설명으로 한자가 "
                    em { "왜 이 모양이고 왜 이 뜻인지" }
                    "를 이해할 수 있게 합니다. 위키피디아처럼 꼬리에 꼬리를 무는 사전형 탐색 경험을 지향해요."
                }
                p {
                    "한국 한자음과 뜻은 전제가 아니라 보조 비계입니다 — 한자를 아는 분에게는 보너스, 모르는 분에게는 부담이 없도록."
                }
            }

            // ── 방법론: 데이터 소스 위계 (설계 문서 8장) ─────────
            section { class: "section",
                h2 { class: "section__title", "데이터는 어디서 오나" }
                p {
                    "종류별로 1차 출처를 정해 두고, 신뢰 수준이 다른 데이터를 섞지 않습니다. 어원 서술은 학자마다 견해가 갈리므로 항상 "
                    em { "해석" }
                    "임을 명시하고 출처를 함께 적습니다."
                }
                div { class: "about__table-wrap",
                    table { class: "about__table",
                        thead {
                            tr {
                                th { "데이터 종류" }
                                th { "1차 출처" }
                                th { "라이선스" }
                                th { "신뢰도" }
                            }
                        }
                        tbody {
                            for (kind, source, license, trust) in DATA_SOURCES {
                                tr {
                                    td { "{kind}" }
                                    td { "{source}" }
                                    td { "{license}" }
                                    td { "{trust}" }
                                }
                            }
                        }
                    }
                }
            }

            // ── 신뢰도 라벨 (설계 문서 5장) ──────────────────────
            section { class: "section",
                h2 { class: "section__title", "신뢰도 라벨의 의미" }
                p { "모든 한자 페이지 상단에는 어원 서술의 신뢰도 배지가 붙습니다." }
                ul { class: "about__labels",
                    li {
                        span { class: "confidence confidence--well about__label-badge",
                            span { class: "confidence__label", "근거 확실" }
                        }
                        span { class: "about__label-desc",
                            "(well-attested) 갑골문·금문 등 자료로 뒷받침되어 정설로 널리 받아들여지는 설명입니다."
                        }
                    }
                    li {
                        span { class: "confidence confidence--interp about__label-badge",
                            span { class: "confidence__label", "해석" }
                        }
                        span { class: "about__label-desc",
                            "(interpretation) 여러 학설 중 하나를 골라 소개한 것입니다. 다른 견해가 있을 수 있어요."
                        }
                    }
                    li {
                        span { class: "confidence confidence--tentative about__label-badge",
                            span { class: "confidence__label", "잠정" }
                        }
                        span { class: "about__label-desc",
                            "(tentative) 근거가 약한 잠정적 추정입니다. 참고로만 읽어 주세요."
                        }
                    }
                }
            }

            // ── 한계 ─────────────────────────────────────────────
            section { class: "section",
                h2 { class: "section__title", "지금의 한계" }
                ul { class: "about__list",
                    li { "MVP 단계라 등재 한자가 아직 적습니다 (N5 핵심 + 어원이 풍부한 한자부터 채우는 중)." }
                    li { "자형 변천 글리프(갑골문·금문 등)는 자료가 있는 한자만 표시하고, 없으면 해당 단계를 생략합니다." }
                    li { "부수 어원 서술은 핵심 부수부터 깊이 있게 채우고 있으며, 나머지는 골격만 있을 수 있습니다." }
                    li { "어원 설명은 본질적으로 해석입니다 — 단정이 아니라 가장 설득력 있는 이야기를 고른 것입니다." }
                }
            }

            // ── 기여 안내 (여정 C 요약) ──────────────────────────
            section { class: "section",
                h2 { class: "section__title", "틀린 곳을 찾으셨나요?" }
                p {
                    "어원 설명이 이상하거나 더 나은 학설을 아신다면 제보를 환영합니다. 각 한자 페이지 하단의 "
                    strong { "\"정정 제안\"" }
                    " 버튼으로 알려 주세요 (준비 중). 제안은 검토를 거쳐 콘텐츠에 반영되고, 사이트는 자동으로 다시 배포됩니다."
                }
                p { "개발자라면 GitHub 저장소에 이슈를 직접 남기셔도 좋습니다." }
                Link { class: "button-link", to: Route::BrowsePage {}, "한자 둘러보기" }
            }
        }
    }
}

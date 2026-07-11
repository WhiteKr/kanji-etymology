//! `/browse` 둘러보기 페이지 (M6) — 전체 한자 그리드 + JLPT/획수/부수 필터.
//!
//! 데이터는 kanji-list.json + by-radical.json 두 개뿐이라 필터링은 전부
//! 클라이언트 사이드에서 처리한다. 세 필터는 AND로 조합된다.

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::api::{self, FetchError, KanjiSummary};
use crate::Route;

/// JLPT 필터 선택지 (쉬운 급수부터).
const JLPT_LEVELS: [&str; 5] = ["N5", "N4", "N3", "N2", "N1"];

/// 획수 구간 버튼: (표시 문구, 최소, 최대).
const STROKE_RANGES: [(&str, u32, u32); 4] = [
    ("1~4획", 1, 4),
    ("5~8획", 5, 8),
    ("9~12획", 9, 12),
    ("13획~", 13, u32::MAX),
];

/// 페이지에서 쓰는 데이터 묶음 — 한자 요약 목록 + 부수 역인덱스.
#[derive(Clone, PartialEq)]
struct BrowseData {
    list: Vec<KanjiSummary>,
    by_radical: HashMap<String, Vec<String>>,
    /// 필터 버튼 표시용으로 정렬해 둔 부수/부품 키 목록.
    radicals: Vec<String>,
}

#[component]
pub fn BrowsePage() -> Element {
    let data = use_resource(|| async {
        let list = api::fetch_kanji_list().await?;
        let by_radical = api::fetch_by_radical().await?;
        let mut radicals: Vec<String> = by_radical.keys().cloned().collect();
        radicals.sort();
        Ok::<BrowseData, FetchError>(BrowseData {
            list,
            by_radical,
            radicals,
        })
    });

    // 필터 상태 — None이면 "전체".
    let jlpt = use_signal(|| None::<String>);
    let strokes = use_signal(|| None::<(u32, u32)>);
    let radical = use_signal(|| None::<String>);

    rsx! {
        document::Title { "둘러보기 — 한자 어원 사전" }
        main { class: "page browse-page",
            h1 { class: "page-title", "둘러보기" }
            p { class: "page-lead", "등재된 전체 한자를 JLPT 급수·획수·부수로 걸러 볼 수 있어요." }

            match &*data.read() {
                None => rsx! {
                    p { class: "status-message", "불러오는 중…" }
                },
                Some(Err(err)) => rsx! {
                    section { class: "status-block",
                        h1 { "한자 목록을 불러오지 못했습니다" }
                        p { "{err}" }
                    }
                },
                Some(Ok(data)) => rsx! {
                    BrowseFilters { data: data.clone(), jlpt, strokes, radical }
                    BrowseResults { data: data.clone(), jlpt, strokes, radical }
                },
            }
        }
    }
}

/// 필터 칩 3줄 — JLPT / 획수 구간 / 부수.
#[component]
fn BrowseFilters(
    data: BrowseData,
    jlpt: Signal<Option<String>>,
    strokes: Signal<Option<(u32, u32)>>,
    radical: Signal<Option<String>>,
) -> Element {
    rsx! {
        div { class: "filters",
            // ── JLPT ────────────────────────────────────────────
            div { class: "filter-group",
                span { class: "filter-group__label", "JLPT" }
                FilterChip {
                    label: "전체".to_string(),
                    active: jlpt().is_none(),
                    onselect: move |_| jlpt.set(None),
                }
                for level in JLPT_LEVELS {
                    FilterChip {
                        label: level.to_string(),
                        active: jlpt().as_deref() == Some(level),
                        onselect: move |_| jlpt.set(Some(level.to_string())),
                    }
                }
            }

            // ── 획수 구간 ───────────────────────────────────────
            div { class: "filter-group",
                span { class: "filter-group__label", "획수" }
                FilterChip {
                    label: "전체".to_string(),
                    active: strokes().is_none(),
                    onselect: move |_| strokes.set(None),
                }
                for (label, min, max) in STROKE_RANGES {
                    FilterChip {
                        label: label.to_string(),
                        active: strokes() == Some((min, max)),
                        onselect: move |_| strokes.set(Some((min, max))),
                    }
                }
            }

            // ── 부수/부품 (by-radical 키) ───────────────────────
            div { class: "filter-group",
                span { class: "filter-group__label", "부수" }
                FilterChip {
                    label: "전체".to_string(),
                    active: radical().is_none(),
                    onselect: move |_| radical.set(None),
                }
                for r in data.radicals.iter() {
                    {
                        let r = r.clone();
                        let is_active = radical().as_deref() == Some(r.as_str());
                        rsx! {
                            FilterChip {
                                label: r.clone(),
                                active: is_active,
                                hanja: true,
                                onselect: move |_| radical.set(Some(r.clone())),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 필터 칩 버튼 하나.
#[component]
fn FilterChip(
    label: String,
    active: bool,
    #[props(default = false)] hanja: bool,
    onselect: EventHandler<()>,
) -> Element {
    let mut class = String::from("chip");
    if active {
        class.push_str(" chip--active");
    }
    if hanja {
        class.push_str(" hanja");
    }
    rsx! {
        button {
            class: "{class}",
            r#type: "button",
            aria_pressed: "{active}",
            onclick: move |_| onselect.call(()),
            "{label}"
        }
    }
}

/// 필터 적용 결과 그리드 (0건이면 안내 + 초기화 버튼).
#[component]
fn BrowseResults(
    data: BrowseData,
    jlpt: Signal<Option<String>>,
    strokes: Signal<Option<(u32, u32)>>,
    radical: Signal<Option<String>>,
) -> Element {
    // 부수 필터가 걸려 있으면 해당 부품을 가진 한자 집합을 먼저 구한다.
    // 역인덱스에 없는 부수라면 빈 집합 → 결과 0건 안내로 이어진다.
    let radical_members: Option<Vec<String>> = radical
        .read()
        .as_ref()
        .map(|r| data.by_radical.get(r).cloned().unwrap_or_default());

    let filtered: Vec<&KanjiSummary> = data
        .list
        .iter()
        .filter(|item| {
            // JLPT 필터 — 급수 미상(None)은 특정 급수 선택 시 제외된다.
            if let Some(sel) = jlpt.read().as_deref() {
                if item.jlpt.as_deref() != Some(sel) {
                    return false;
                }
            }
            // 획수 필터 — 획수 미상은 구간 선택 시 제외된다.
            if let Some((min, max)) = *strokes.read() {
                match item.stroke_count {
                    Some(n) if (min..=max).contains(&n) => {}
                    _ => return false,
                }
            }
            // 부수 필터.
            if let Some(members) = &radical_members {
                if !members.iter().any(|c| c == &item.character) {
                    return false;
                }
            }
            true
        })
        .collect();

    let count = filtered.len();
    let has_filter = jlpt.read().is_some() || strokes.read().is_some() || radical.read().is_some();

    if filtered.is_empty() {
        return rsx! {
            section { class: "status-block",
                h1 { "조건에 맞는 한자가 없습니다" }
                p { "필터 조합을 바꾸거나 초기화해 보세요. 콘텐츠는 계속 추가되고 있습니다." }
                button {
                    class: "button-link browse__reset",
                    r#type: "button",
                    onclick: move |_| {
                        jlpt.set(None);
                        strokes.set(None);
                        radical.set(None);
                    },
                    "필터 초기화"
                }
            }
        };
    }

    rsx! {
        section { class: "section browse-results",
            p { class: "browse-results__summary",
                if has_filter {
                    "조건에 맞는 한자 {count}자"
                } else {
                    "전체 {count}자"
                }
            }
            div { class: "kanji-grid",
                for item in filtered {
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

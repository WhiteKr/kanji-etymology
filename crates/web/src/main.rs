//! 한자 어원 사전 — Dioxus 웹 앱 진입점 (M4 앱 코어).
//!
//! 라우트 구조는 설계 문서 6장(정보 구조) 참조:
//! `docs/2026-05-27-kanji-etymology-mvp-design.md`

use dioxus::prelude::*;

mod api;
mod pages;
mod search;
mod search_modal;

use pages::{AboutPage, BrowsePage, KanjiPage, Landing, NotFound, RadicalPage, RadicalsPage, SearchPage};
use search_modal::SearchModal;

/// 전체 페이지 CSS (모바일 퍼스트, CSS 변수 테마).
static MAIN_CSS: Asset = asset!("/assets/main.css");

/// 앱 라우트. URL의 한자(`/kanji/学`)는 브라우저에서 percent-encoding되지만
/// dioxus-router가 디코딩해서 `character` 프롭으로 넘겨준다.
///
/// 라우트 필드는 `String`이지만 페이지 컴포넌트는 `ReadSignal<String>`으로
/// 받는다(rsx 프롭 자동 변환). `/kanji/学` → `/kanji/子`처럼 같은 라우트에서
/// 파라미터만 바뀔 때 `use_resource`가 변경을 구독해 다시 fetch하게 하기 위함.
#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
enum Route {
    // 모든 페이지를 사이트 공통 레이아웃(헤더 + 검색 모달)으로 감싼다.
    #[layout(SiteLayout)]
        #[route("/")]
        Landing {},

        #[route("/kanji/:character")]
        KanjiPage { character: String },

        // 부수 페이지 — 부수 어원 + 해당 부수를 가진 한자 목록 (M6).
        #[route("/radical/:radical")]
        RadicalPage { radical: String },

        // 부수 인덱스 — 전체 부수 일람 (M6).
        #[route("/radicals")]
        RadicalsPage {},

        // 둘러보기 — 전체 한자 그리드 + JLPT/획수/부수 필터 (M6).
        #[route("/browse")]
        BrowsePage {},

        // 소개 — 방법론·출처·한계·기여 안내 (M6).
        #[route("/about")]
        AboutPage {},

        // 검색 결과 페이지 (M5). `q`는 쿼리 파라미터 — 없으면 빈 문자열.
        #[route("/search?:q")]
        SearchPage { q: String },

        // catch-all 404 — 친절한 안내 + 홈 링크 (비슷한 한자 추천은 M6).
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // 한자 표시용 Noto Sans JP 웹폰트 (구글 폰트, font-display: swap).
        // 한글 본문은 시스템 폰트 스택을 쓴다 (main.css 참조).
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link {
            rel: "preconnect",
            href: "https://fonts.gstatic.com",
            crossorigin: "anonymous",
        }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Noto+Sans+JP:wght@400;500;700&display=swap",
        }
        document::Stylesheet { href: MAIN_CSS }

        Router::<Route> {}
    }
}

/// 사이트 공통 레이아웃 — 상단 헤더(브랜드 + 검색 버튼) + 본문 + 검색 모달.
/// 어느 페이지에서든 헤더 버튼 또는 `/` 단축키로 검색 모달을 열 수 있다.
#[component]
fn SiteLayout() -> Element {
    // `/` 단축키 리스너 — 입력 필드에 포커스가 없을 때만 모달을 연다.
    // 레이아웃은 앱 수명 내내 마운트돼 있으므로 리스너 해제는 필요 없다.
    use_effect(|| {
        spawn(async move {
            let mut shortcut = document::eval(
                r#"
                document.addEventListener("keydown", (e) => {
                    const tag = document.activeElement ? document.activeElement.tagName : "";
                    if (e.key === "/" && !e.isComposing
                        && tag !== "INPUT" && tag !== "TEXTAREA" && tag !== "SELECT") {
                        e.preventDefault();
                        dioxus.send(true);
                    }
                });
                "#,
            );
            while shortcut.recv::<bool>().await.is_ok() {
                search_modal::open_search();
            }
        });
    });

    rsx! {
        header { class: "site-header",
            div { class: "site-header__inner",
                Link { class: "site-header__brand", to: Route::Landing {}, "한자 어원 사전" }
                // 주 내비게이션 — 모바일에서는 글자 크기를 줄여 한 줄 유지 (main.css).
                nav { class: "site-header__nav", aria_label: "주 메뉴",
                    Link { class: "site-header__nav-link", to: Route::BrowsePage {}, "둘러보기" }
                    Link { class: "site-header__nav-link", to: Route::RadicalsPage {}, "부수" }
                    Link { class: "site-header__nav-link", to: Route::AboutPage {}, "소개" }
                }
                button {
                    class: "site-header__search",
                    r#type: "button",
                    aria_label: "검색 열기",
                    onclick: move |_| search_modal::open_search(),
                    span { class: "site-header__search-label", "검색" }
                    kbd { class: "site-header__kbd", "/" }
                }
            }
        }

        Outlet::<Route> {}

        SearchModal {}
    }
}

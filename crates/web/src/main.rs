//! 한자 어원 사전 — Dioxus 웹 앱 진입점 (M4 앱 코어).
//!
//! 라우트 구조는 설계 문서 6장(정보 구조) 참조:
//! `docs/2026-05-27-kanji-etymology-mvp-design.md`

use dioxus::prelude::*;

mod api;
mod pages;

use pages::{KanjiPage, Landing, NotFound, RadicalPage};

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
    #[route("/")]
    Landing {},

    #[route("/kanji/:character")]
    KanjiPage { character: String },

    // 부수 페이지 본 구현은 M6. 지금은 자리만 잡아 둔다 (components 링크 대상).
    #[route("/radical/:radical")]
    RadicalPage { radical: String },

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

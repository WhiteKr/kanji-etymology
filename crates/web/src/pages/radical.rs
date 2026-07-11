//! 부수 페이지 자리 표시 — 본 구현(어원 서술 + 해당 부수 한자 목록)은 M6.
//! 한자 페이지의 부품 카드가 `/radical/{부수}`로 링크하므로 라우트만 먼저 잡아 둔다.

use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn RadicalPage(radical: ReadSignal<String>) -> Element {
    rsx! {
        document::Title { "{radical} — 한자 어원 사전" }
        main { class: "page",
            section { class: "status-block",
                h1 { "부수 페이지는 준비 중입니다" }
                p {
                    span { class: "hanja status-block__char", "{radical}" }
                    " 부수의 어원과 이 부수를 가진 한자 목록을 준비하고 있어요."
                }
                Link { class: "button-link", to: Route::Landing {}, "홈으로 돌아가기" }
            }
        }
    }
}

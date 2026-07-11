//! catch-all 404 페이지 — 친절한 안내 + 홈 링크.
//! 비슷한 한자 추천은 M6에서 추가한다.

use dioxus::prelude::*;

use crate::Route;

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
                p { "홈에서 등재된 한자를 둘러보실 수 있어요." }
                Link { class: "button-link", to: Route::Landing {}, "홈으로 돌아가기" }
            }
        }
    }
}

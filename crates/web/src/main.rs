use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        main {
            h1 { "한자 어원 사전" }
            p { "M1 스캐폴딩 — 구현 준비 중입니다." }
        }
    }
}

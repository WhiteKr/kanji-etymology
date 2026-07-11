//! 정정 제안(피드백) 모달 (M7 — 설계 문서 4장 여정 C · 11장).
//!
//! 한자 페이지의 "이의 제기 / 정정 제안" 버튼으로 열리는 인앱 폼.
//! 제출 흐름: 클라이언트 검증 → Turnstile 토큰 → Cloudflare Worker(`feedback-worker`)
//! → GitHub Issue 자동 생성. 요청/응답 계약은 `crates/feedback-worker/src/logic.rs`의
//! `FeedbackRequest` / `SuccessBody` / `ErrorBody`와 일치해야 한다.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// ── 배포 상수 ─────────────────────────────────────────────────
// 실서비스 값 교체는 이 블록만 수정하면 된다.

/// 피드백 Worker 엔드포인트 (POST, JSON).
/// Worker는 `ALLOWED_ORIGIN`(GitHub Pages 도메인)만 허용하므로
/// 로컬 `dx serve`에서 제출하면 403이 돌아오는 것이 정상이다.
pub const FEEDBACK_ENDPOINT: &str = "https://kanji-feedback.whitekr.workers.dev/feedback";

/// Cloudflare Turnstile 사이트 키.
///
/// 현재 값은 Cloudflare 공식 **테스트 키**(항상 통과, 가시 위젯)다.
/// 실키 교체 방법: Cloudflare 대시보드 → Turnstile → 위젯 생성(도메인:
/// `whitekr.github.io`) 후 발급된 sitekey를 여기 붙여넣고, Worker 쪽
/// `TURNSTILE_SECRET` secret도 짝이 되는 시크릿 키로 갱신한다.
pub const TURNSTILE_SITE_KEY: &str = "1x00000000000000000000AA";

/// 콘텐츠 저장소 Issues URL — 성공 안내 링크와 "직접 제보" 보조 링크에 사용.
pub const GITHUB_ISSUES_URL: &str = "https://github.com/WhiteKr/kanji-etymology/issues";

// ── 어원 설명 해시 ────────────────────────────────────────────

/// FNV-1a 64bit 해시 → 소문자 16자리 hex 문자열.
///
/// 제출 시점에 사용자가 보고 있던 어원 설명(body_markdown)을 식별하기 위한
/// 값(`current_explanation_hash`). 암호학적 강도는 필요 없고, 콘텐츠 개정 전후를
/// 구분할 수만 있으면 된다. UTF-8 바이트 단위로 계산하므로 결정적이다.
pub fn fnv1a_64_hex(input: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

// ── 요청/응답 DTO (feedback-worker logic.rs와 계약 일치) ──────

/// `POST /feedback` 요청 본문 — `FeedbackRequest`(logic.rs)와 필드 동일.
#[derive(Debug, Serialize)]
struct FeedbackPayload {
    kanji: String,
    current_explanation_hash: String,
    suggestion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    contact: Option<String>,
    turnstile_token: String,
    /// 허니팟 — 사람에게는 보이지 않는 입력. 값이 있으면 Worker가 거절한다.
    website: String,
}

/// 성공 응답 `{"issue_number": N}` — `SuccessBody`(logic.rs) 대응.
#[derive(Debug, Deserialize)]
struct SuccessResponse {
    issue_number: u64,
}

/// 실패 응답 `{"error": "..."}` — `ErrorBody`(logic.rs) 대응.
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

/// 제출 상태 머신.
#[derive(Debug, Clone, PartialEq)]
enum SubmitState {
    /// 입력 중 (초기 상태).
    Editing,
    /// 전송 중 — 제출 버튼 비활성.
    Sending,
    /// 성공 — 생성된 Issue 번호.
    Sent { issue_number: u64 },
    /// 실패 — 사용자에게 보여줄 한국어 안내.
    Failed(String),
}

/// Worker로 제안을 전송한다. 실패 시 상태 코드별 한국어 안내를 돌려준다.
async fn send_feedback(payload: &FeedbackPayload) -> Result<u64, String> {
    let request = gloo_net::http::Request::post(FEEDBACK_ENDPOINT)
        .json(payload)
        .map_err(|e| format!("요청을 만들지 못했습니다: {e}"))?;

    let resp = request
        .send()
        .await
        .map_err(|_| "네트워크 오류가 발생했습니다. 인터넷 연결을 확인해 주세요.".to_string())?;

    if resp.ok() {
        let body: SuccessResponse = resp
            .json()
            .await
            .map_err(|e| format!("응답을 해석하지 못했습니다: {e}"))?;
        return Ok(body.issue_number);
    }

    // 실패 — Worker가 준 한국어 메시지(400 검증 실패 등)를 우선 사용하고,
    // 없으면 상태 코드별 기본 안내로 대체한다.
    let status = resp.status();
    let server_message = resp.json::<ErrorResponse>().await.ok().map(|b| b.error);
    Err(match status {
        429 => "제보가 너무 잦습니다. 잠시 후 다시 시도해 주세요.".to_string(),
        403 => "허용되지 않은 접속 경로입니다. 보안 확인에 실패했거나, 정식 사이트(whitekr.github.io)가 아닌 곳에서는 제출할 수 없습니다.".to_string(),
        400 => server_message
            .unwrap_or_else(|| "입력 내용을 다시 확인해 주세요.".to_string()),
        502 => "제보 저장에 실패했습니다. 잠시 후 다시 시도하거나 GitHub Issue로 직접 제보해 주세요.".to_string(),
        other => server_message
            .unwrap_or_else(|| format!("알 수 없는 오류가 발생했습니다 (HTTP {other}). 잠시 후 다시 시도해 주세요.")),
    })
}

// ── Turnstile 연동 (동적 로드 + 명시적 렌더) ──────────────────

/// Turnstile 위젯을 렌더할 컨테이너 div의 id.
const TURNSTILE_CONTAINER_ID: &str = "feedback-turnstile";

/// api.js를 동적 로드하고 명시적 렌더링(`turnstile.render`)으로 위젯을 붙인다.
/// 토큰 발급·만료·오류는 `dioxus.send`로 Rust 쪽 시그널에 전달된다
/// (빈 문자열 = 토큰 없음). `format!` 중괄호 이스케이프를 피하려고
/// 사이트 키 주입부와 본문을 분리해 이어붙인다.
const TURNSTILE_BOOTSTRAP_JS: &str = r#"
const el = document.getElementById("feedback-turnstile");
function renderWidget() {
    if (!el || !window.turnstile) return;
    el.innerHTML = "";
    // 위젯 id를 보관해 두면 제출 실패 시 reset으로 새 토큰을 받을 수 있다.
    window.__feedbackTurnstileId = window.turnstile.render(el, {
        sitekey: SITE_KEY,
        callback: (token) => dioxus.send(token),
        "expired-callback": () => dioxus.send(""),
        "error-callback": () => dioxus.send(""),
    });
}
if (window.turnstile) {
    renderWidget();
} else {
    let script = document.getElementById("turnstile-script");
    if (!script) {
        script = document.createElement("script");
        script.id = "turnstile-script";
        script.src = "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";
        script.async = true;
        document.head.appendChild(script);
    }
    script.addEventListener("load", renderWidget);
}
"#;

/// 제출 실패 후 Turnstile 위젯을 리셋한다 (실키에서는 토큰이 1회용이므로 필수).
const TURNSTILE_RESET_JS: &str = r#"
if (window.turnstile && window.__feedbackTurnstileId !== undefined) {
    try { window.turnstile.reset(window.__feedbackTurnstileId); } catch (e) {}
}
"#;

// ── 컴포넌트 ──────────────────────────────────────────────────

/// 모달 래퍼 — 닫혀 있으면 아무것도 렌더하지 않는다 (search_modal 패턴).
/// 열릴 때마다 내부 컴포넌트가 새로 마운트되어 입력·상태가 초기화된다.
#[component]
pub fn FeedbackModal(kanji: String, explanation_hash: String, open: Signal<bool>) -> Element {
    if !open() {
        return rsx! {};
    }
    rsx! {
        FeedbackModalInner { kanji, explanation_hash, open }
    }
}

#[component]
fn FeedbackModalInner(kanji: String, explanation_hash: String, mut open: Signal<bool>) -> Element {
    let mut suggestion = use_signal(String::new);
    let mut contact = use_signal(String::new);
    // 허니팟 값 — 사람은 건드리지 않으므로 항상 빈 문자열이어야 한다.
    let mut website = use_signal(String::new);
    // Turnstile이 발급한 토큰. 빈 문자열이면 아직 확인 전(제출 불가).
    let mut turnstile_token = use_signal(String::new);
    let mut state = use_signal(|| SubmitState::Editing);

    // 마운트 후 Turnstile 위젯을 렌더하고 토큰 콜백을 수신한다.
    // 컴포넌트가 언마운트되면 spawn 태스크가 함께 드롭되어 수신이 멈춘다.
    use_effect(move || {
        spawn(async move {
            let js = format!(r#"const SITE_KEY = "{TURNSTILE_SITE_KEY}";"#) + TURNSTILE_BOOTSTRAP_JS;
            let mut eval = document::eval(&js);
            while let Ok(token) = eval.recv::<String>().await {
                turnstile_token.set(token);
            }
        });
    });

    // 클라이언트 검증 — Worker(logic.rs)와 동일한 규칙(10~4000자, chars 기준).
    let char_count = suggestion().chars().count();
    let suggestion_valid = (10..=4000).contains(&char_count);
    let has_token = !turnstile_token().is_empty();
    let sending = state() == SubmitState::Sending;
    let can_submit = suggestion_valid && has_token && !sending;

    // 제출 핸들러 — String 프롭은 호출마다 clone해서 async로 넘긴다.
    let submit_kanji = kanji.clone();
    let submit_hash = explanation_hash.clone();
    let on_submit = move |_| {
        if !suggestion_valid || turnstile_token().is_empty() || state() == SubmitState::Sending {
            return;
        }
        let contact_value = contact().trim().to_string();
        let payload = FeedbackPayload {
            kanji: submit_kanji.clone(),
            current_explanation_hash: submit_hash.clone(),
            suggestion: suggestion(),
            contact: (!contact_value.is_empty()).then_some(contact_value),
            turnstile_token: turnstile_token(),
            // 허니팟은 값 그대로 전송 — 봇이 채웠다면 Worker가 거절한다.
            website: website(),
        };
        spawn(async move {
            state.set(SubmitState::Sending);
            match send_feedback(&payload).await {
                Ok(issue_number) => state.set(SubmitState::Sent { issue_number }),
                Err(message) => {
                    state.set(SubmitState::Failed(message));
                    // 실키에서는 토큰이 1회용 — 리셋해서 새 토큰을 받는다.
                    turnstile_token.set(String::new());
                    document::eval(TURNSTILE_RESET_JS);
                }
            }
        });
    };

    rsx! {
        // 바깥(오버레이) 클릭·ESC로 닫기 — 패널 클릭은 전파를 끊는다.
        // keydown은 포커스된 자식(텍스트 영역 등)에서 버블링돼 올라온다.
        div {
            class: "feedback-modal__overlay",
            onclick: move |_| open.set(false),
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    open.set(false);
                }
            },
            div {
                class: "feedback-modal",
                role: "dialog",
                aria_label: "정정 제안",
                onclick: move |e| e.stop_propagation(),

                header { class: "feedback-modal__header",
                    span { class: "hanja feedback-modal__char", "{kanji}" }
                    div { class: "feedback-modal__heading",
                        h2 { class: "feedback-modal__title", "정정 제안" }
                        p { class: "feedback-modal__intro",
                            "어원 설명은 여러 학설을 바탕으로 한 해석입니다. "
                            "다른 견해나 오류 제보를 환영합니다."
                        }
                    }
                    button {
                        class: "feedback-modal__close",
                        r#type: "button",
                        aria_label: "닫기",
                        onclick: move |_| open.set(false),
                        "×"
                    }
                }

                match state() {
                    SubmitState::Sent { issue_number } => rsx! {
                        div { class: "feedback-modal__success",
                            p { class: "feedback-modal__success-title",
                                "감사합니다. 24시간 내 검토합니다."
                            }
                            p { class: "feedback-modal__success-detail",
                                "제보가 "
                                a {
                                    href: "{GITHUB_ISSUES_URL}/{issue_number}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    "Issue #{issue_number}"
                                }
                                "로 등록되었습니다."
                            }
                            button {
                                class: "feedback-modal__submit",
                                r#type: "button",
                                onclick: move |_| open.set(false),
                                "닫기"
                            }
                        }
                    },
                    _ => rsx! {
                        div { class: "feedback-modal__form",
                            label { class: "feedback-modal__label", r#for: "feedback-suggestion",
                                "제안 내용"
                            }
                            textarea {
                                id: "feedback-suggestion",
                                class: "feedback-modal__textarea",
                                placeholder: "어느 부분이 어떻게 다르다고 생각하시는지 적어 주세요. 근거나 출처가 있다면 함께 남겨 주시면 큰 도움이 됩니다.",
                                value: "{suggestion}",
                                disabled: sending,
                                // 동적으로 삽입되는 요소라 autofocus 대신 직접 포커스.
                                onmounted: move |e| async move {
                                    let _ = e.set_focus(true).await;
                                },
                                oninput: move |e| suggestion.set(e.value()),
                            }
                            p {
                                class: if char_count > 0 && !suggestion_valid {
                                    "feedback-modal__counter feedback-modal__counter--invalid"
                                } else {
                                    "feedback-modal__counter"
                                },
                                if char_count > 0 && char_count < 10 {
                                    "10자 이상 입력해 주세요 · "
                                }
                                if char_count > 4000 {
                                    "4000자를 초과했습니다 · "
                                }
                                "{char_count} / 4000"
                            }

                            label { class: "feedback-modal__label", r#for: "feedback-contact",
                                "연락처"
                            }
                            input {
                                id: "feedback-contact",
                                class: "feedback-modal__input",
                                r#type: "email",
                                placeholder: "답변 받을 이메일 (선택)",
                                value: "{contact}",
                                disabled: sending,
                                oninput: move |e| contact.set(e.value()),
                            }

                            // 허니팟 — 화면 밖으로 밀어내 숨긴다 (display:none이 아니라
                            // DOM에 살아 있어 봇 자동 완성이 채우도록 유도).
                            div { class: "feedback-modal__website", aria_hidden: "true",
                                label { r#for: "website", "Website" }
                                input {
                                    id: "website",
                                    name: "website",
                                    r#type: "text",
                                    tabindex: "-1",
                                    autocomplete: "off",
                                    value: "{website}",
                                    oninput: move |e| website.set(e.value()),
                                }
                            }

                            // Turnstile 위젯 컨테이너 — use_effect의 JS가 채운다.
                            div { id: TURNSTILE_CONTAINER_ID, class: "feedback-modal__turnstile" }
                            if !has_token {
                                p { class: "feedback-modal__hint",
                                    "봇 확인이 완료되면 제출할 수 있습니다."
                                }
                            }

                            if let SubmitState::Failed(message) = state() {
                                p { class: "feedback-modal__error", role: "alert", "{message}" }
                            }

                            button {
                                class: "feedback-modal__submit",
                                r#type: "button",
                                disabled: !can_submit,
                                onclick: on_submit,
                                if sending { "제출 중…" } else { "제안 보내기" }
                            }

                            a {
                                class: "feedback-modal__github",
                                href: "{GITHUB_ISSUES_URL}/new",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                "🔗 GitHub Issue로 직접 제보"
                            }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // FNV-1a 64bit 공식 테스트 벡터
    // (http://www.isthe.com/chongo/tech/comp/fnv/ 참조).

    #[test]
    fn fnv1a_64_hex_empty_string_is_offset_basis() {
        assert_eq!(fnv1a_64_hex(""), "cbf29ce484222325");
    }

    #[test]
    fn fnv1a_64_hex_matches_known_vectors() {
        assert_eq!(fnv1a_64_hex("a"), "af63dc4c8601ec8c");
        assert_eq!(fnv1a_64_hex("foobar"), "85944171f73967e8");
    }

    #[test]
    fn fnv1a_64_hex_is_deterministic_for_utf8() {
        let markdown = "「学」은 지붕 아래에서 아이가 배우는 모습입니다.";
        assert_eq!(fnv1a_64_hex(markdown), fnv1a_64_hex(markdown));
    }

    #[test]
    fn fnv1a_64_hex_distinguishes_revisions() {
        assert_ne!(fnv1a_64_hex("설명 v1"), fnv1a_64_hex("설명 v2"));
    }

    #[test]
    fn fnv1a_64_hex_is_16_lowercase_hex_digits() {
        for input in ["", "a", "学", "긴 마크다운 본문…"] {
            let hash = fnv1a_64_hex(input);
            assert_eq!(hash.len(), 16, "input={input:?}");
            assert!(
                hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "input={input:?} hash={hash}"
            );
        }
    }

    #[test]
    fn feedback_payload_serializes_to_worker_contract() {
        let payload = FeedbackPayload {
            kanji: "学".to_string(),
            current_explanation_hash: fnv1a_64_hex("본문"),
            suggestion: "제안 내용입니다 열 글자 이상.".to_string(),
            contact: None,
            turnstile_token: "token123".to_string(),
            website: String::new(),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["kanji"], "学");
        assert_eq!(json["turnstile_token"], "token123");
        assert_eq!(json["website"], "");
        // contact가 None이면 필드 자체를 생략한다 (Worker의 #[serde(default)] 대응).
        assert!(json.get("contact").is_none());
    }
}

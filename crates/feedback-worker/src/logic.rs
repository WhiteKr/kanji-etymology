//! 순수 로직 — Cloudflare Worker 바인딩(`worker` crate)과 완전히 분리된 모듈.
//!
//! 이 모듈은 어떤 wasm/워커 전용 의존성도 사용하지 않으므로
//! `cargo test -p feedback-worker` 로 host 타겟에서 그대로 단위 테스트할 수 있다.
//! (설계 문서 11장 / 구현 계획 M7 참조)

use serde::{Deserialize, Serialize};

/// 제안 내용 최소 길이 (글자 수).
pub const MIN_SUGGESTION_LEN: usize = 10;
/// 제안 내용 최대 길이 (글자 수).
pub const MAX_SUGGESTION_LEN: usize = 4000;
/// IP당 허용되는 시간당 요청 수.
pub const RATE_LIMIT_PER_HOUR: u32 = 5;
/// Rate limit 카운터의 KV TTL(초).
pub const RATE_LIMIT_TTL_SECONDS: u64 = 3600;

/// `POST /feedback` 요청 본문.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FeedbackRequest {
    pub kanji: String,
    pub current_explanation_hash: String,
    pub suggestion: String,
    #[serde(default)]
    pub contact: Option<String>,
    pub turnstile_token: String,
    /// 봇 방지용 허니팟 필드. 사람 사용자에게는 보이지 않아야 하며, 값이 채워져 있으면
    /// 봇으로 간주한다.
    #[serde(default)]
    pub website: Option<String>,
}

/// 본문 검증 실패 사유.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    Honeypot,
    SuggestionTooShort,
    SuggestionTooLong,
    InvalidKanji,
}

impl ValidationError {
    /// 사용자에게 보여줄 한국어 에러 메시지.
    pub fn message(self) -> &'static str {
        match self {
            // 봇에게 정확한 실패 사유를 알려주지 않기 위해 일반적인 메시지를 사용한다.
            ValidationError::Honeypot => "요청을 처리할 수 없습니다.",
            ValidationError::SuggestionTooShort => "제안 내용은 최소 10자 이상 입력해 주세요.",
            ValidationError::SuggestionTooLong => "제안 내용은 4000자를 초과할 수 없습니다.",
            ValidationError::InvalidKanji => "kanji 필드는 정확히 한 글자여야 합니다.",
        }
    }

    /// 이 에러에 대응하는 HTTP 상태 코드.
    pub fn status_code(self) -> u16 {
        400
    }
}

/// 허니팟 · 길이 · kanji 형식을 검증한다. Turnstile · rate limit · GitHub 호출 이전에 수행한다.
pub fn validate_feedback(req: &FeedbackRequest) -> Result<(), ValidationError> {
    if let Some(website) = &req.website {
        if !website.trim().is_empty() {
            return Err(ValidationError::Honeypot);
        }
    }

    let suggestion_len = req.suggestion.chars().count();
    if suggestion_len < MIN_SUGGESTION_LEN {
        return Err(ValidationError::SuggestionTooShort);
    }
    if suggestion_len > MAX_SUGGESTION_LEN {
        return Err(ValidationError::SuggestionTooLong);
    }

    if req.kanji.chars().count() != 1 {
        return Err(ValidationError::InvalidKanji);
    }

    Ok(())
}

/// wrangler.toml `ALLOWED_ORIGIN` 값(쉼표로 구분된 목록 허용)과 요청의 `Origin` 헤더를 비교한다.
pub fn is_allowed_origin(allowed_origins: &str, origin: &str) -> bool {
    allowed_origins
        .split(',')
        .map(|s| s.trim())
        .any(|allowed| !allowed.is_empty() && allowed == origin)
}

/// GitHub Issue 제목: `[제보] {kanji} — {suggestion 앞 40자}`.
pub fn build_issue_title(kanji: &str, suggestion: &str) -> String {
    let truncated: String = suggestion.chars().take(40).collect();
    format!("[제보] {kanji} — {truncated}")
}

/// GitHub Issue 본문(마크다운). 한자 · 기존 설명 해시 · 제안 · 연락처를 포함한다.
pub fn build_issue_body(req: &FeedbackRequest) -> String {
    let contact = req
        .contact
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .unwrap_or("(제공 안 함)");

    format!(
        "## 한자\n{}\n\n## 기존 설명 해시\n`{}`\n\n## 제안 내용\n{}\n\n## 연락처\n{}\n",
        req.kanji, req.current_explanation_hash, req.suggestion, contact
    )
}

/// GitHub Issue 라벨: `["feedback", "from-web", "kanji:{kanji}"]`.
pub fn build_issue_labels(kanji: &str) -> Vec<String> {
    vec!["feedback".to_string(), "from-web".to_string(), format!("kanji:{kanji}")]
}

/// GitHub Issues API `POST /repos/{repo}/issues` 요청 본문.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GithubIssuePayload {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

/// [`FeedbackRequest`]로부터 GitHub Issue 생성 페이로드를 구성한다.
pub fn build_issue_payload(req: &FeedbackRequest) -> GithubIssuePayload {
    GithubIssuePayload {
        title: build_issue_title(&req.kanji, &req.suggestion),
        body: build_issue_body(req),
        labels: build_issue_labels(&req.kanji),
    }
}

/// GitHub Issues API 응답에서 필요한 부분만 추출.
#[derive(Debug, Clone, Deserialize)]
pub struct GithubIssueResponse {
    pub number: u64,
}

/// Cloudflare Turnstile `siteverify` 응답.
/// <https://developers.cloudflare.com/turnstile/get-started/server-side-validation/>
#[derive(Debug, Clone, Deserialize)]
pub struct TurnstileVerifyResponse {
    pub success: bool,
    #[serde(rename = "error-codes", default)]
    pub error_codes: Vec<String>,
}

/// 성공 응답 `{"issue_number": N}`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SuccessBody {
    pub issue_number: u64,
}

/// 실패 응답 `{"error": "..."}`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ErrorBody {
    pub error: String,
}

impl ErrorBody {
    pub fn new(message: impl Into<String>) -> Self {
        Self { error: message.into() }
    }
}

/// IP당 rate limit 카운터의 KV 키.
pub fn rate_limit_key(ip: &str) -> String {
    format!("ratelimit:{ip}")
}

/// 현재 카운트가 시간당 한도를 초과했는지 여부.
pub fn is_rate_limited(current_count: u32) -> bool {
    current_count >= RATE_LIMIT_PER_HOUR
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> FeedbackRequest {
        FeedbackRequest {
            kanji: "学".to_string(),
            current_explanation_hash: "abc123".to_string(),
            suggestion: "이 설명에는 오류가 있습니다. 자세히 설명하면...".to_string(),
            contact: Some("test@example.com".to_string()),
            turnstile_token: "token".to_string(),
            website: None,
        }
    }

    // --- validate_feedback ---

    #[test]
    fn validate_feedback_accepts_valid_request() {
        assert!(validate_feedback(&valid_request()).is_ok());
    }

    #[test]
    fn validate_feedback_rejects_filled_honeypot() {
        let mut req = valid_request();
        req.website = Some("http://spam.example".to_string());
        assert_eq!(validate_feedback(&req), Err(ValidationError::Honeypot));
    }

    #[test]
    fn validate_feedback_accepts_blank_honeypot() {
        let mut req = valid_request();
        req.website = Some("   ".to_string());
        assert!(validate_feedback(&req).is_ok());
    }

    #[test]
    fn validate_feedback_rejects_short_suggestion() {
        let mut req = valid_request();
        req.suggestion = "짧음".to_string();
        assert_eq!(
            validate_feedback(&req),
            Err(ValidationError::SuggestionTooShort)
        );
    }

    #[test]
    fn validate_feedback_accepts_min_length_suggestion() {
        let mut req = valid_request();
        req.suggestion = "가".repeat(MIN_SUGGESTION_LEN);
        assert!(validate_feedback(&req).is_ok());
    }

    #[test]
    fn validate_feedback_rejects_too_long_suggestion() {
        let mut req = valid_request();
        req.suggestion = "가".repeat(MAX_SUGGESTION_LEN + 1);
        assert_eq!(
            validate_feedback(&req),
            Err(ValidationError::SuggestionTooLong)
        );
    }

    #[test]
    fn validate_feedback_accepts_max_length_suggestion() {
        let mut req = valid_request();
        req.suggestion = "가".repeat(MAX_SUGGESTION_LEN);
        assert!(validate_feedback(&req).is_ok());
    }

    #[test]
    fn validate_feedback_rejects_multi_char_kanji() {
        let mut req = valid_request();
        req.kanji = "学生".to_string();
        assert_eq!(validate_feedback(&req), Err(ValidationError::InvalidKanji));
    }

    #[test]
    fn validate_feedback_rejects_empty_kanji() {
        let mut req = valid_request();
        req.kanji = String::new();
        assert_eq!(validate_feedback(&req), Err(ValidationError::InvalidKanji));
    }

    #[test]
    fn validation_error_messages_are_korean_and_non_empty() {
        for err in [
            ValidationError::Honeypot,
            ValidationError::SuggestionTooShort,
            ValidationError::SuggestionTooLong,
            ValidationError::InvalidKanji,
        ] {
            assert!(!err.message().is_empty());
            assert_eq!(err.status_code(), 400);
        }
    }

    // --- CORS origin whitelist ---

    #[test]
    fn is_allowed_origin_matches_exact_single_origin() {
        assert!(is_allowed_origin(
            "https://example.github.io",
            "https://example.github.io"
        ));
    }

    #[test]
    fn is_allowed_origin_rejects_mismatch() {
        assert!(!is_allowed_origin(
            "https://example.github.io",
            "https://evil.example"
        ));
    }

    #[test]
    fn is_allowed_origin_supports_comma_separated_list() {
        let allowed = "https://a.example, https://b.example";
        assert!(is_allowed_origin(allowed, "https://a.example"));
        assert!(is_allowed_origin(allowed, "https://b.example"));
        assert!(!is_allowed_origin(allowed, "https://c.example"));
    }

    #[test]
    fn is_allowed_origin_rejects_empty_config() {
        assert!(!is_allowed_origin("", "https://example.github.io"));
    }

    // --- Issue title/body/labels formatting ---

    #[test]
    fn build_issue_title_truncates_to_40_chars() {
        let long_suggestion = "가".repeat(100);
        let title = build_issue_title("学", &long_suggestion);
        assert!(title.starts_with("[제보] 学 — "));
        // "[제보] 学 — " 접두어를 제외한 나머지가 40자인지 확인.
        let suffix: String = title.chars().skip("[제보] 学 — ".chars().count()).collect();
        assert_eq!(suffix.chars().count(), 40);
    }

    #[test]
    fn build_issue_title_keeps_short_suggestion_intact() {
        let title = build_issue_title("学", "짧은 제안");
        assert_eq!(title, "[제보] 学 — 짧은 제안");
    }

    #[test]
    fn build_issue_body_includes_all_fields() {
        let req = valid_request();
        let body = build_issue_body(&req);
        assert!(body.contains(&req.kanji));
        assert!(body.contains(&req.current_explanation_hash));
        assert!(body.contains(&req.suggestion));
        assert!(body.contains("test@example.com"));
    }

    #[test]
    fn build_issue_body_shows_placeholder_when_no_contact() {
        let mut req = valid_request();
        req.contact = None;
        let body = build_issue_body(&req);
        assert!(body.contains("(제공 안 함)"));
    }

    #[test]
    fn build_issue_body_treats_blank_contact_as_missing() {
        let mut req = valid_request();
        req.contact = Some("   ".to_string());
        let body = build_issue_body(&req);
        assert!(body.contains("(제공 안 함)"));
    }

    #[test]
    fn build_issue_labels_includes_kanji_label() {
        let labels = build_issue_labels("学");
        assert_eq!(labels, vec!["feedback", "from-web", "kanji:学"]);
    }

    #[test]
    fn build_issue_payload_matches_individual_builders() {
        let req = valid_request();
        let payload = build_issue_payload(&req);
        assert_eq!(payload.title, build_issue_title(&req.kanji, &req.suggestion));
        assert_eq!(payload.body, build_issue_body(&req));
        assert_eq!(payload.labels, build_issue_labels(&req.kanji));
    }

    // --- Rate limit ---

    #[test]
    fn rate_limit_key_is_namespaced_by_ip() {
        assert_eq!(rate_limit_key("1.2.3.4"), "ratelimit:1.2.3.4");
        assert_ne!(rate_limit_key("1.2.3.4"), rate_limit_key("5.6.7.8"));
    }

    #[test]
    fn is_rate_limited_allows_up_to_limit() {
        for count in 0..RATE_LIMIT_PER_HOUR {
            assert!(!is_rate_limited(count), "count={count} should be allowed");
        }
    }

    #[test]
    fn is_rate_limited_blocks_at_and_above_limit() {
        assert!(is_rate_limited(RATE_LIMIT_PER_HOUR));
        assert!(is_rate_limited(RATE_LIMIT_PER_HOUR + 1));
    }

    // --- Serde round trips ---

    #[test]
    fn feedback_request_deserializes_from_expected_json() {
        let json = r#"{
            "kanji": "学",
            "current_explanation_hash": "abc123",
            "suggestion": "제안 내용입니다 열 글자 이상.",
            "contact": "test@example.com",
            "turnstile_token": "token123"
        }"#;
        let req: FeedbackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.kanji, "学");
        assert_eq!(req.website, None);
    }

    #[test]
    fn feedback_request_deserializes_honeypot_field() {
        let json = r#"{
            "kanji": "学",
            "current_explanation_hash": "abc123",
            "suggestion": "제안 내용입니다 열 글자 이상.",
            "turnstile_token": "token123",
            "website": "http://spam.example"
        }"#;
        let req: FeedbackRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.website.as_deref(), Some("http://spam.example"));
    }

    #[test]
    fn turnstile_verify_response_parses_success() {
        let json = r#"{"success": true, "challenge_ts": "2022-02-28T15:14:30.096Z", "hostname": "example.com", "error-codes": []}"#;
        let resp: TurnstileVerifyResponse = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        assert!(resp.error_codes.is_empty());
    }

    #[test]
    fn turnstile_verify_response_parses_failure() {
        let json = r#"{"success": false, "error-codes": ["invalid-input-response"]}"#;
        let resp: TurnstileVerifyResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.error_codes, vec!["invalid-input-response".to_string()]);
    }

    #[test]
    fn github_issue_response_parses_number() {
        let json = r#"{"number": 42, "id": 999, "title": "ignored"}"#;
        let resp: GithubIssueResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.number, 42);
    }

    #[test]
    fn success_body_serializes_expected_shape() {
        let body = SuccessBody { issue_number: 7 };
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, r#"{"issue_number":7}"#);
    }

    #[test]
    fn error_body_serializes_expected_shape() {
        let body = ErrorBody::new("문제가 발생했습니다.");
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, r#"{"error":"문제가 발생했습니다."}"#);
    }
}

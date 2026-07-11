//! Cloudflare Worker 엔트리포인트 (wasm32 타겟 전용).
//!
//! 요청/응답, KV, 외부 fetch 등 `worker` crate 바인딩만 다루고,
//! 실제 검증 · 포맷팅 로직은 전부 [`crate::logic`]에 위임한다.
//! 이 모듈은 `Cargo.toml`에서 `worker` 의존성이 wasm32 타겟에만 걸려 있으므로
//! host 빌드(`cargo build --workspace`)에는 전혀 포함되지 않는다.

use worker::wasm_bindgen::JsValue;
use worker::*;

use crate::logic::{
    build_issue_payload, is_allowed_origin, is_rate_limited, rate_limit_key, validate_feedback,
    ErrorBody, FeedbackRequest, GithubIssuePayload, GithubIssueResponse, SuccessBody,
    TurnstileVerifyResponse, RATE_LIMIT_TTL_SECONDS,
};

const TURNSTILE_VERIFY_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const CORS_ALLOWED_METHODS: &str = "POST, OPTIONS";
const CORS_ALLOWED_HEADERS: &str = "Content-Type";
const CORS_MAX_AGE_SECONDS: &str = "86400";
const GITHUB_USER_AGENT: &str = "kanji-feedback-worker";

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .options("/feedback", handle_options)
        .post_async("/feedback", handle_feedback)
        .run(req, env)
        .await
}

/// `OPTIONS /feedback` — CORS preflight 응답.
fn handle_options(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let origin = request_origin(&req)?;
    let allowed_origin = ctx.var("ALLOWED_ORIGIN")?.to_string();

    let mut response = Response::empty()?.with_status(204);
    apply_cors_headers(&mut response, &allowed_origin, origin.as_deref())?;
    response
        .headers()
        .set("Access-Control-Max-Age", CORS_MAX_AGE_SECONDS)?;
    Ok(response)
}

/// `POST /feedback` — CORS 검증 → 본문 검증 → Turnstile → rate limit → GitHub Issue 생성.
async fn handle_feedback(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let origin = request_origin(&req)?;
    let allowed_origin = ctx.var("ALLOWED_ORIGIN")?.to_string();

    let origin_ok = origin
        .as_deref()
        .map(|o| is_allowed_origin(&allowed_origin, o))
        .unwrap_or(false);
    if !origin_ok {
        return error_response(403, "허용되지 않은 출처입니다.", &allowed_origin, origin.as_deref());
    }

    // 1. 본문 파싱
    let body: FeedbackRequest = match req.json().await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                400,
                "요청 본문을 파싱할 수 없습니다.",
                &allowed_origin,
                origin.as_deref(),
            );
        }
    };

    // 2. honeypot · 길이 · kanji 형식 검증
    if let Err(validation_error) = validate_feedback(&body) {
        return error_response(
            validation_error.status_code(),
            validation_error.message(),
            &allowed_origin,
            origin.as_deref(),
        );
    }

    // 3. Turnstile 토큰 검증
    let turnstile_secret = ctx.secret("TURNSTILE_SECRET")?.to_string();
    let connecting_ip = req
        .headers()
        .get("CF-Connecting-IP")?
        .unwrap_or_else(|| "unknown".to_string());

    match verify_turnstile(&turnstile_secret, &body.turnstile_token, &connecting_ip).await {
        Ok(true) => {}
        Ok(false) => {
            return error_response(
                403,
                "캡차 인증에 실패했습니다.",
                &allowed_origin,
                origin.as_deref(),
            );
        }
        Err(_) => {
            return error_response(
                502,
                "캡차 검증 서버에 연결할 수 없습니다.",
                &allowed_origin,
                origin.as_deref(),
            );
        }
    }

    // 4. Rate limit — IP당 시간당 5건 (Workers KV, TTL 1시간 카운터)
    let kv = ctx.kv("RATE_LIMIT_KV")?;
    let key = rate_limit_key(&connecting_ip);
    let current_count: u32 = kv
        .get(&key)
        .text()
        .await?
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(0);

    if is_rate_limited(current_count) {
        return error_response(
            429,
            "요청이 너무 많습니다. 잠시 후 다시 시도해 주세요.",
            &allowed_origin,
            origin.as_deref(),
        );
    }

    kv.put(&key, (current_count + 1).to_string())?
        .expiration_ttl(RATE_LIMIT_TTL_SECONDS)
        .execute()
        .await?;

    // 5. GitHub Issue 생성
    let github_pat = ctx.secret("GITHUB_PAT")?.to_string();
    let github_repo = ctx.var("GITHUB_REPO")?.to_string();
    let payload = build_issue_payload(&body);

    match create_github_issue(&github_repo, &github_pat, &payload).await {
        Ok(issue_number) => {
            let success = SuccessBody { issue_number };
            let mut response = Response::from_json(&success)?;
            apply_cors_headers(&mut response, &allowed_origin, origin.as_deref())?;
            Ok(response)
        }
        Err(_) => error_response(
            502,
            "GitHub Issue 생성에 실패했습니다.",
            &allowed_origin,
            origin.as_deref(),
        ),
    }
}

/// 요청의 `Origin` 헤더를 읽는다.
fn request_origin(req: &Request) -> Result<Option<String>> {
    req.headers().get("Origin")
}

/// 응답에 CORS 헤더를 적용한다. `origin`이 화이트리스트에 있을 때만
/// `Access-Control-Allow-Origin`을 설정한다(허용되지 않은 출처에는 값을 반환하지 않음).
fn apply_cors_headers(
    response: &mut Response,
    allowed_origin_var: &str,
    origin: Option<&str>,
) -> Result<()> {
    if let Some(origin) = origin {
        if is_allowed_origin(allowed_origin_var, origin) {
            response.headers().set("Access-Control-Allow-Origin", origin)?;
            response.headers().set("Vary", "Origin")?;
        }
    }
    response
        .headers()
        .set("Access-Control-Allow-Methods", CORS_ALLOWED_METHODS)?;
    response
        .headers()
        .set("Access-Control-Allow-Headers", CORS_ALLOWED_HEADERS)?;
    Ok(())
}

/// 표준화된 JSON 에러 응답 + CORS 헤더.
fn error_response(
    status: u16,
    message: &str,
    allowed_origin_var: &str,
    origin: Option<&str>,
) -> Result<Response> {
    let body = ErrorBody::new(message);
    let mut response = Response::from_json(&body)?.with_status(status);
    apply_cors_headers(&mut response, allowed_origin_var, origin)?;
    Ok(response)
}

/// Cloudflare Turnstile `siteverify` 호출.
/// <https://developers.cloudflare.com/turnstile/get-started/server-side-validation/>
async fn verify_turnstile(secret: &str, token: &str, remote_ip: &str) -> Result<bool> {
    #[derive(serde::Serialize)]
    struct TurnstileVerifyRequestBody<'a> {
        secret: &'a str,
        response: &'a str,
        remoteip: &'a str,
    }

    let payload = TurnstileVerifyRequestBody {
        secret,
        response: token,
        remoteip: remote_ip,
    };
    let body = serde_json::to_string(&payload).map_err(|e| Error::RustError(e.to_string()))?;

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.method = Method::Post;
    init.headers = headers;
    init.body = Some(JsValue::from_str(&body));

    let request = Request::new_with_init(TURNSTILE_VERIFY_URL, &init)?;
    let mut response = Fetch::Request(request).send().await?;
    let verify: TurnstileVerifyResponse = response.json().await?;
    Ok(verify.success)
}

/// GitHub Issues API `POST /repos/{repo}/issues` 호출. 생성된 이슈 번호를 반환한다.
async fn create_github_issue(
    repo: &str,
    pat: &str,
    payload: &GithubIssuePayload,
) -> Result<u64> {
    let url = format!("https://api.github.com/repos/{repo}/issues");
    let body = serde_json::to_string(payload).map_err(|e| Error::RustError(e.to_string()))?;

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {pat}"))?;
    headers.set("Accept", "application/vnd.github+json")?;
    headers.set("Content-Type", "application/json")?;
    headers.set("User-Agent", GITHUB_USER_AGENT)?;
    headers.set("X-GitHub-Api-Version", "2022-11-28")?;

    let mut init = RequestInit::new();
    init.method = Method::Post;
    init.headers = headers;
    init.body = Some(JsValue::from_str(&body));

    let request = Request::new_with_init(&url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() >= 300 {
        let error_text = response.text().await.unwrap_or_default();
        return Err(Error::RustError(format!(
            "GitHub API 오류: status={} body={error_text}",
            response.status_code()
        )));
    }

    let issue: GithubIssueResponse = response.json().await?;
    Ok(issue.number)
}

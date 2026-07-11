//! 피드백 프록시 — Cloudflare Worker (Rust, workers-rs).
//!
//! 인앱 정정 제안 폼 → Turnstile 검증 → rate limit → GitHub Issue 생성.
//! 설계: docs/2026-05-27-kanji-etymology-mvp-design.md 11장
//! 구현: M7 마일스톤 (docs/2026-07-11-implementation-plan.md)
//!
//! ## 모듈 구성
//! - [`logic`]: `worker` crate 바인딩과 완전히 분리된 순수 로직(검증 · 포맷팅 · rate limit 키
//!   계산). host 타겟에서 `cargo test -p feedback-worker`로 바로 테스트할 수 있다.
//! - `worker_handler` (wasm32 타겟 전용): 실제 Cloudflare Worker 엔트리포인트
//!   (`#[event(fetch)]`). `worker` crate는 `Cargo.toml`에서 wasm32 타겟에만 의존성으로
//!   걸려 있으므로(target-gated dependency), host 빌드(`cargo build --workspace`)에는
//!   영향을 주지 않는다.

pub mod logic;

#[cfg(target_arch = "wasm32")]
mod worker_handler;

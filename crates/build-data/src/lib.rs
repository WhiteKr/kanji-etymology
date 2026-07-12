//! 빌드 시 데이터 파이프라인 라이브러리.
//!
//! `content/kanji/*.md`, `content/radicals/*.md` frontmatter를 파싱·검증하고
//! 검색 인덱스·역인덱스·개별 JSON을 생성한다. 검증 실패 시 어떤 파일도
//! 쓰지 않고 오류를 모아 반환한다 — 잘못된 데이터로 배포되는 일을 막기 위함.
//!
//! `main.rs`는 CLI 인자 파싱과 종료 코드 결정만 담당하는 얇은 진입점이고,
//! 실제 로직은 전부 여기(라이브러리)에 있어 `cargo test`로 함수 단위 검증이
//! 가능하다.
//!
//! 설계: `docs/2026-05-27-kanji-etymology-mvp-design.md` 7·9·10장
//! 구현 계획: `docs/2026-07-11-implementation-plan.md` M2절

pub mod frontmatter;
pub mod import_kanjidic;
pub mod index;
pub mod kana;
pub mod kanjidic;
pub mod pipeline;
pub mod skeleton;

pub use pipeline::{run, Summary};

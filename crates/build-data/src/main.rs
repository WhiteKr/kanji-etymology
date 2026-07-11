//! 빌드 시 데이터 파이프라인.
//!
//! `content/kanji/*.md` frontmatter를 파싱·검증하고
//! 검색 인덱스(`dist/data/search-index.json`)를 생성한다.
//! 검증 실패 시 종료 코드 1로 빌드를 중단시킨다.
//!
//! 구현: M2 마일스톤 (docs/2026-07-11-implementation-plan.md)

fn main() {
    eprintln!("build-data: 아직 구현되지 않음 (M2)");
    std::process::exit(1);
}

# 한자 어원 학습 모듈 — MVP 구현 계획

| 항목 | 값 |
|------|----|
| 작성일 | 2026-07-11 |
| 상태 | Active |
| 선행 문서 | [MVP 설계 문서](2026-05-27-kanji-etymology-mvp-design.md) |
| 다음 단계 | 마일스톤 1부터 순차 구현 |

---

## 0. 확정 사항 (설계 문서에서 미정이었던 것)

**스타일링: 일반 CSS (모바일 퍼스트, CSS 변수 기반 테마)**

- Full Rust 스택에 Node 툴체인(Tailwind)을 추가할 이유가 없음 — 빌드 파이프라인 단순 유지
- `stylist`는 Yew 생태계 중심이라 Dioxus와 궁합이 나쁨
- MVP 화면 수(~8 라우트)에서 유틸리티 프레임워크의 이점이 크지 않음
- 다크 모드(Phase 2)를 대비해 색상은 처음부터 CSS 변수로 정의

## 1. 마일스톤 개요

```
M1 스캐폴딩 ──▶ M2 데이터 파이프라인 ──▶ M3 샘플 콘텐츠
                                          │
                              ┌───────────┴───────────┐
                              ▼                       ▼
                        M4 앱 코어              M7 피드백 Worker (병렬)
                              │
                              ▼
                        M5 검색
                              │
                              ▼
                        M6 나머지 페이지
                              │
                              ▼
                        M8 PWA + CI/CD
                              │
                              ▼
                        M9 콘텐츠 50자 + 검증
```

M7(피드백 Worker)은 프론트엔드와 독립적이므로 M4~M6과 병렬 진행 가능.

## 2. 마일스톤 상세

### M1 — 워크스페이스 스캐폴딩

부록 A 구조대로 Cargo 워크스페이스 생성.

- [ ] 루트 `Cargo.toml` 워크스페이스 + `crates/web`(Dioxus), `crates/build-data`(bin), `crates/feedback-worker`(workers-rs)
- [ ] `content/kanji/`, `content/radicals/`, `public/` 디렉터리
- [ ] Dioxus CLI로 `crates/web` hello-world가 로컬에서 뜨는 것 확인
- [ ] Rust 툴체인 핀 (`rust-toolchain.toml`), Dioxus 버전 핀

**완료 기준**: `cargo build` 워크스페이스 전체 성공, `dx serve`로 빈 앱 구동.

### M2 — 데이터 모델 + `build-data` 파이프라인

frontmatter 스키마(설계 7장)를 Rust 타입으로 정의하고 파싱·검증·인덱스 생성 구현. 스키마가 굳어야 콘텐츠·프론트엔드가 병렬로 갈 수 있으므로 최우선.

- [ ] `KanjiEntry` 등 스키마 타입 (`serde` + `serde_yaml`) — web 크레이트와 공유하도록 별도 모듈/크레이트 검토
- [ ] `content/kanji/*.md` 파싱: frontmatter 분리 + 타입 검증 + 필수 필드 확인
- [ ] 참조 무결성 검증 (components/related 링크 대상 존재 확인)
- [ ] 검색 인덱스 생성 (`by_kanji`, `by_kr_sound`, `by_meaning`, `by_on`, `by_kun`, `by_romaji`)
- [ ] 역인덱스 생성 (부수→한자, 단어→한자)
- [ ] 출력: `dist/data/search-index.json` + 한자별 개별 JSON

**완료 기준**: 정상 fixture는 통과, 깨진 fixture(필수 필드 누락·죽은 링크)는 **빌드 실패**하는 `cargo test` 존재.

### M3 — 샘플 콘텐츠 5~10자

UI 개발용 실데이터. 50자 전체는 M9에서.

- [ ] 어원이 풍부한 N5 한자 5~10자 선정 (예: 学, 子, 日, 月, 明, 休, 木, 山)
- [ ] AI 초안 → 본인 검수 파이프라인(설계 10장) 1회전 실행, 프롬프트 템플릿을 `docs/`에 기록
- [ ] 핵심 부수 3~5개 (`content/radicals/`)

**완료 기준**: `build-data`가 샘플 콘텐츠로 검증 통과 + 인덱스 생성.

### M4 — Dioxus 앱 코어

- [ ] 라우터 + GitHub Pages SPA 트릭 (`404.html` → hash 보존 리다이렉트 → 클라이언트 복구)
- [ ] 한자 페이지 (설계 5장 레이아웃): 헤더 → 자형 변천 → 부품 분해 → 어원 서술(마크다운 렌더) → 더 깊이 섹션
- [ ] 신뢰도 배지 (well-attested / interpretation / tentative)
- [ ] 랜딩 페이지 (검색 바 자리, 오늘의 한자, 최근 추가)
- [ ] 기본 CSS: 모바일 퍼스트, CSS 변수, 터치 44px, Noto Sans JP 서브셋

**완료 기준**: 샘플 한자 페이지가 모바일·데스크톱에서 설계 5장 레이아웃대로 렌더, 부품 클릭 시 해당 페이지로 이동.

### M5 — 검색 (5가지 진입 경로)

- [ ] 검색 모달 + 인덱스 lazy fetch + 메모리 캐시
- [ ] 매칭: 정확 → 접두 → 부분 일치 우선순위
- [ ] 한국어 자모 normalize, 로마자→가나 변환
- [ ] `/search?q=` 결과 페이지 (카드 그리드)

**완료 기준**: `学`·`학`·`배우다`·`まなぶ`·`manabu` 다섯 입력 모두 学에 도달.

### M6 — 나머지 페이지

- [ ] `/browse` (JLPT/획수/부수 필터)
- [ ] `/radical/{부수}`, `/radicals`
- [ ] `/about` (방법론·출처·한계·기여 안내)
- [ ] 친절한 404 (비슷한 한자 추천)

**완료 기준**: 설계 6장 라우트 표의 전 경로 동작.

### M7 — 피드백 Worker (M4~M6과 병렬 가능)

- [ ] `workers-rs` 엔드포인트: CORS 화이트리스트 → Turnstile 검증 → rate limit(KV) → validation → GitHub Issues API
- [ ] 프론트 정정 제안 모달 (한자·설명 해시 프리필) + GitHub 직접 링크
- [ ] fine-grained PAT (Issues:write, 해당 레포 한정) Worker secret 등록

**완료 기준**: 웹 폼 제출 → 실제 GitHub Issue 생성 E2E 1회 성공, rate limit 초과 시 429 확인.

### M8 — PWA + CI/CD

- [ ] `manifest.json` + 아이콘 세트
- [ ] 서비스 워커 (JS): 방문 한자 페이지·검색 인덱스 캐시, 오프라인 동작
- [ ] 랜딩 + 인기 한자 SSG prerender
- [ ] GitHub Actions: `build-data` → Dioxus 빌드 → Pages 배포 (rust-cache 적용), Worker 배포 워크플로 분리

**완료 기준**: main push → 자동 배포. Lighthouse PWA 설치 가능 판정, 오프라인에서 캐시된 페이지 열림, LCP < 3초(모바일 4G 스로틀).

### M9 — 콘텐츠 50자 + 최종 검증

- [ ] N5 핵심 + 어원 풍부한 한자 50자 목록 확정 → 파이프라인으로 작성·검수
- [ ] 핵심 부수 ~20개 어원 서술, 나머지 골격
- [ ] 설계 16장 성공 기준 체크리스트 전 항목 검증
- [ ] 피드백 → Issue → PR 반영 → 자동 재배포 사이클 1회 실증
- [ ] 외부 시범 사용자 5명 모집

**완료 기준**: 설계 문서 16장 성공 기준 전부 체크.

## 3. 마일스톤별 커밋 규칙

- 마일스톤 하나 = PR 하나가 이상적이나 개인 프로젝트이므로 main 직접 커밋 허용, 단 **마일스톤 완료 기준 통과 후 태그** (`m1`, `m2`, …)
- 콘텐츠 커밋(`content/`)과 코드 커밋은 분리
- CI가 생긴 M8 이후에는 빌드 깨지는 커밋 금지

## 4. 첫 작업

**M1 착수**: 워크스페이스 스캐폴딩 → `dx serve` 확인까지.

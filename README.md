# 한자 어원 학습 모듈 (가제)

[![deploy](https://github.com/WhiteKr/kanji-etymology/actions/workflows/deploy.yml/badge.svg)](https://github.com/WhiteKr/kanji-etymology/actions/workflows/deploy.yml)

한국인 일본어 학습자를 위한, **어원 스토리와 부수 분해 중심의 한자 사전**.
한자를 *외우는* 게 아니라 *이해*하는 경험을 목표로 합니다.

**배포**: <https://whitekr.github.io/kanji-etymology/> (main push 시 자동 배포)

> ✅ **현재 상태**: MVP 배포됨 (한자 50자, M1~M8 + 피드백 파이프라인 E2E 검증 완료).
> 남은 것: 콘텐츠 검수([검수 노트](docs/content-review-notes.md)), Turnstile 실키 교체(현재 테스트 키 — 캡차가 실제로 봇을 막지 않음), 외부 시범 사용자.

## 피드백 파이프라인 (운영)

웹 폼 → Cloudflare Worker(`kanji-feedback.whitekr.workers.dev`) → Turnstile 검증 → KV rate limit(IP당 5건/시간) → GitHub Issue 자동 생성(`feedback` 라벨).
제보 검토 후 `content/` 수정 → push → 자동 재배포.

Turnstile 실키 교체: [대시보드](https://dash.cloudflare.com/?to=/:account/turnstile)에서 위젯 생성 후
`crates/web/src/feedback_modal.rs`의 `TURNSTILE_SITE_KEY` 교체 + `wrangler secret put TURNSTILE_SECRET`.

## 로컬 개발

```powershell
# 1. 콘텐츠 → 데이터 JSON 생성 (검증 포함, 최초 1회 및 콘텐츠 변경 시)
cargo run -p build-data -- --content-dir content --out-dir crates/web/assets/data

# 2. 개발 서버 (Dioxus CLI 필요: cargo binstall dioxus-cli)
dx serve -p kanji-web
# base_path 설정으로 http://127.0.0.1:8080/kanji-etymology/ 에서 열림

# 테스트
cargo test --workspace
```

## 콘텐츠 확장: KANJIDIC2 임포트 (초안 스켈레톤 생성)

새 한자 콘텐츠를 처음부터 손으로 쓰는 대신, [KANJIDIC2](http://www.edrdg.org/wiki/index.php/KANJIDIC_Project)에서
기계적 필드(음독·훈독·한국음·획수·학년·영어 의미)를 뽑아 `content/kanji/{글자}.md` 형식의
**초안 스켈레톤**을 생성할 수 있습니다. 어원 서술(본문)은 사람/AI가 나중에 채웁니다.

```powershell
# KANJIDIC2 XML은 아래 배포처에서 직접 내려받아야 합니다 (이 도구는 다운로드하지 않음).
# http://www.edrdg.org/wiki/index.php/KANJIDIC_Project

cargo run -p build-data -- import-kanjidic `
  --xml kanjidic2.xml `
  --chars 一二三 `
  --out-dir drafts `
  --skip-existing --content-dir content

# 옵션 전체: cargo run -p build-data -- import-kanjidic --help
```

생성된 스켈레톤은 `jlpt`/`korean.meaning`/`last_updated` 등 KANJIDIC2에 없는 값을
`"TODO"` placeholder로, 영어 의미는 `meanings`에 `"(en) ..."` 접두로 임시 표기합니다
(한국어 번역·어원 서술 필요). 따라서 **`build-data`의 콘텐츠 검증을 통과하지 못합니다** —
TODO를 실제 값으로 채우고 어원 서술을 작성한 뒤 `content/kanji/`로 옮겨야 정식 콘텐츠가 됩니다.

> **KANJIDIC2 라이선스**: 전자사전연구개발그룹(EDRDG)이 CC-BY-SA 4.0으로 배포합니다.
> 이 저장소는 KANJIDIC2 XML을 포함하거나 배포하지 않으며, `import-kanjidic`도 네트워크에서
> 내려받지 않습니다 — 사용자가 위 배포처에서 직접 받은 로컬 파일 경로를 `--xml`로 넘겨야
> 합니다. 생성된 스켈레톤의 `sources`에는 KANJIDIC2 출처·라이선스가 자동으로 기록됩니다.

## 문서

- [MVP 디자인 (2026-05-27)](docs/2026-05-27-kanji-etymology-mvp-design.md)
- [MVP 구현 계획 (2026-07-11)](docs/2026-07-11-implementation-plan.md)

## 기술 스택 (계획)

- **프론트엔드**: Dioxus (Rust → WASM)
- **백엔드 (피드백 프록시)**: Cloudflare Workers + `workers-rs` (Rust)
- **빌드 데이터 파이프라인**: 자체 Rust 바이너리
- **호스팅**: GitHub Pages (정적)
- **피드백 저장소**: GitHub Issues (Worker 경유 자동 생성)
- **콘텐츠 형식**: YAML frontmatter + Markdown (한자 1자 = 파일 1개)

## 폴더 구조

```
.
├── crates/
│   ├── web/                # Dioxus 앱
│   ├── build-data/         # 빌드 시 데이터 파이프라인
│   └── feedback-worker/    # Cloudflare Worker (Rust)
├── content/
│   ├── kanji/              # 한자 .md 파일들
│   └── radicals/           # 부수 정보
├── public/                 # 사이트 루트에 배치할 파일 (sw.js — CI가 번들 루트로 복사)
├── docs/                   # 설계 문서
└── .github/workflows/      # CI/CD
```

## 라이선스

미정 (구현 단계에서 결정).

# 한자 어원 학습 모듈 (가제)

[![deploy](https://github.com/WhiteKr/kanji-etymology/actions/workflows/deploy.yml/badge.svg)](https://github.com/WhiteKr/kanji-etymology/actions/workflows/deploy.yml)

한국인 일본어 학습자를 위한, **어원 스토리와 부수 분해 중심의 한자 사전**.
한자를 *외우는* 게 아니라 *이해*하는 경험을 목표로 합니다.

**배포**: <https://whitekr.github.io/kanji-etymology/> (main push 시 자동 배포)

> 🚧 **현재 상태**: 구현 중 — M8 (PWA + CI/CD). [구현 계획](docs/2026-07-11-implementation-plan.md) 참조.

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

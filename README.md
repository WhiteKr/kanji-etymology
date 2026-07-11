# 한자 어원 학습 모듈 (가제)

한국인 일본어 학습자를 위한, **어원 스토리와 부수 분해 중심의 한자 사전**.
한자를 *외우는* 게 아니라 *이해*하는 경험을 목표로 합니다.

> 📐 **현재 상태**: 디자인 단계. 구현은 아직 시작 전.

## 설계 문서

- [MVP 디자인 (2026-05-27)](docs/2026-05-27-kanji-etymology-mvp-design.md)

## 기술 스택 (계획)

- **프론트엔드**: Dioxus (Rust → WASM)
- **백엔드 (피드백 프록시)**: Cloudflare Workers + `workers-rs` (Rust)
- **빌드 데이터 파이프라인**: 자체 Rust 바이너리
- **호스팅**: GitHub Pages (정적)
- **피드백 저장소**: GitHub Issues (Worker 경유 자동 생성)
- **콘텐츠 형식**: YAML frontmatter + Markdown (한자 1자 = 파일 1개)

## 폴더 구조 (예정)

```
.
├── crates/
│   ├── web/                # Dioxus 앱
│   ├── build-data/         # 빌드 시 데이터 파이프라인
│   └── feedback-worker/    # Cloudflare Worker (Rust)
├── content/
│   ├── kanji/              # 한자 .md 파일들
│   └── radicals/           # 부수 정보
├── public/                 # PWA manifest, 아이콘, 폰트 등
├── docs/                   # 설계 문서
└── .github/workflows/      # CI/CD
```

## 라이선스

미정 (구현 단계에서 결정).

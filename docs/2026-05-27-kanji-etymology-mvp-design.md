# 한자 어원 학습 모듈 — MVP 설계 문서

| 항목 | 값 |
|------|----|
| 작성일 | 2026-05-27 |
| 상태 | Draft |
| 다음 단계 | 구현 계획 수립 |
| 상위 비전 | 종합 일본어 학습 서비스의 첫 번째 하위 프로젝트 |

---

## 1. 비전

> **"한자를 외우지 않고 *이해*하는 한국인 학습자를 위한, 어원 스토리와 부수 분해 중심의 한자 사전."**

기존 일본어 학습 서비스의 "암기 위주" 방식 대신, **자형의 역사·부품 분해·서술적 설명**으로 한자가 *왜 이 모양이고 왜 이 뜻인가*를 이해할 수 있게 함. 위키피디아 류의 사전형 탐색 경험.

## 2. 타겟 사용자

**주 타겟**: 한국인 일본어 학습자.

**핵심 전제**: *요즘 한국인은 한자 지식이 약하다.* 따라서 `学生=학생` 같은 한국 한자어 연결을 **전제로 깔지 않는다**. 한국 한자음·뜻은 *전제*가 아니라 *보조 비계(scaffolding)* 로 활용 — 아는 사람에겐 보너스, 모르는 사람에겐 부담 없음.

**진입 레벨**: 분리하지 않고 전 레벨 관통. 각 한자가 자체적으로 깊이를 가지며, 사용자는 원하는 깊이까지 들어감 (점진적 공개).

## 3. 비-목표 (이번 모듈에서 *하지 않는* 것)

- 진도 추적 / 학습 알고리즘 / SRS (간격 반복) — *사전이지 학습 앱이 아님*
- 사용자 계정 / 로그인
- 가나·어휘·문법 모듈 (별도 하위 프로젝트)
- 문장 OCR · 카메라 한자 인식
- 음성 발음 / TTS
- 손글씨 인식
- 다국어 UI (한국어만)
- 모바일 네이티브 앱 (PWA로 충분)
- AI 챗봇 인터페이스

## 4. 핵심 사용자 여정

### 여정 A — 모르는 한자를 만나서 찾아옴 (가장 빈번)
1. 일본어 학습 중 모르는 한자 발견 (예: `学`)
2. 검색 — 한자 직접 / 한국 한자음(`학`) / 일본어 음·훈(`まなぶ`, `gaku`) / 한국어 뜻(`배우다`) 중 어느 것으로든 검색
3. 한자 페이지 진입 → 자형 변천 → 부품 분해 → 어원 서술 순으로 스크롤
4. 부품 클릭 → 그 부품 페이지·관련 한자로 분기

### 여정 B — 호기심 기반 탐색
1. 랜딩 진입 → "오늘의 한자" 또는 큐레이션 카드
2. 카드 클릭 → 어원 서술 → 관련 한자로 이동
3. 위키피디아 토끼굴 패턴

### 여정 C — 어원 설명에 이의 제기 (비개발자 친화)
1. 한자 페이지의 "⚠️ 정정 제안" 클릭
2. 인앱 모달 폼 등장: 한자명·현재 설명 미리 채워짐, 사용자는 제안 텍스트와 (선택) 연락처만 입력
3. 제출 → Cloudflare Worker(Rust) → GitHub Issues API → Issue 자동 생성
4. 사용자에겐 "감사합니다, 24시간 내 검토합니다" 메시지
5. 운영자(=본인)는 GitHub Issues에서 검토 → PR로 콘텐츠 수정 → GitHub Action이 자동 재빌드·재배포
6. (소수의 개발자 사용자를 위한) "🔗 GitHub Issue로 직접" 보조 링크도 함께 제공

## 5. UX — 한자 페이지의 구성

한자 1자 = 1페이지. **어원 스토리(A)와 부품 분해(B)를 통합한 형태**. 위에서 아래로 자연스럽게 깊어짐.

```
─────────────────────────────────
       学  (크게)
       [학] [音 ガク] [訓 まな-ぶ]   배우다 · 학문
─────────────────────────────────
            자형의 변천
   𭃟 → 學 → 學 → 學 → 学
  갑골   금문  전서  정자 신자체
─────────────────────────────────
       구성 — 클릭하여 부품으로 이동
   [⺍ 두 손] + [爻 산가지] + [冖 지붕] + [子 아이]
─────────────────────────────────
            왜 이 모양이 되었나
   (서술형 어원 설명 — 마크다운 본문.
    부품·자형 변천을 엮어 이야기로 풀이.
    한국 한자음, 친척 한자 언급.
    출처 명시.)
─────────────────────────────────
            더 깊이 — 펼쳐 보기
   📚 이 한자가 들어간 단어 (3~5개)
   🔤 음·훈 읽기 패턴 (변음, 連濁 등)
   🔗 같은 부품을 가진 한자
   ⚠️ 이 어원 설명에 이의 제기 / 정정 제안
─────────────────────────────────
```

**원칙**: 어원 서술은 항상 *해석*임을 명시. 신뢰도(`well-attested` / `interpretation` / `tentative`)를 페이지 상단 배지로 표시. 학설이 여럿이면 다중 가설을 보여줌.

## 6. 정보 구조 (라우트)

| 경로 | 페이지 | 설명 |
|------|-------|------|
| `/` | 랜딩 | 검색 바, 오늘의 한자, 큐레이션, 최근 추가 한자 |
| `/kanji/{한자}` | 한자 페이지 | 위 5장의 화면 |
| `/radical/{부수}` | 부수 페이지 | 부수 어원 + 해당 부수 한자 목록 |
| `/radicals` | 부수 인덱스 | 전체 부수 일람 |
| `/search?q=...` | 검색 결과 | 다중 진입 검색 결과 카드 그리드 |
| `/browse` | 둘러보기 | 전체 한자 그리드. JLPT/획수/부수 필터 |
| `/about` | 소개 | 방법론, 출처 일람, 한계, 기여 안내 |
| `/404` | 친절한 안내 | "이 한자는 아직 없음 — 비슷한 한자 추천" |

**SPA 라우팅 (GitHub Pages 정적 제약 대응)**: `404.html`이 `index`로 리다이렉트하면서 path를 hash로 보존 → Dioxus Router가 클라이언트에서 복구. 표준 우회 패턴.

**URL 한자 직접 사용**: `/kanji/学`. 브라우저에서 `/kanji/%E5%AD%A6`로 인코딩되지만 표시는 멀쩡함. SEO·공유 모두 무난.

## 7. 데이터 모델

저장 형식: **YAML frontmatter + Markdown 본문**. 한자 1자 = 파일 1개.

파일 경로: `content/kanji/{character}.md`

```yaml
character: "学"
variants: ["學"]

jlpt: "N5"
grade: 1
stroke_count: 8

korean:
  reading: "학"
  meaning: "배울 학"

readings:
  on: ["ガク"]
  kun:
    - { reading: "まな-ぶ", form: "学ぶ" }

meanings:
  - "배우다"
  - "학문, 공부"

components:
  - { form: "⺍", role: "두 손",   link: null }
  - { form: "爻", role: "산가지", link: "/kanji/爻" }
  - { form: "冖", role: "지붕",   link: "/radical/冖" }
  - { form: "子", role: "아이",   link: "/kanji/子" }

evolution:
  - { era: "갑골문", form: "𭃟",  source: "shuowen" }
  - { era: "금문",   form: "學",  source: "shuowen" }
  - { era: "전서",   form: "學",  source: "shuowen" }
  - { era: "정자",   form: "學",  source: "ja-trad" }
  - { era: "신자체", form: "学",  source: "ja-mod" }

related:
  - { kanji: "字", relation: "동일 부품(子)" }
  - { kanji: "教", relation: "의미적 친척 (가르치다)" }
  - { kanji: "覚", relation: "동일 어원 계열" }

words:
  - { word: "学生", reading: "がくせい", gloss: "학생" }
  - { word: "学校", reading: "がっこう", gloss: "학교" }
  - { word: "学ぶ", reading: "まなぶ",   gloss: "배우다" }

confidence: well-attested   # well-attested | interpretation | tentative

sources:
  - { name: "說文解字", ref: "卷三", year: 121 }
  - { name: "KANJIDIC2", license: "CC-BY-SA", url: "..." }
  - { name: "Wiktionary", url: "https://en.wiktionary.org/wiki/学" }

last_updated: "2026-05-20"
---

# 어원 서술 (마크다운 본문)

원래는 **지붕(冖) 아래에서 아이(子)가 두 손(⺍)으로 산가지(爻)를 잡고
셈을 익히는 장면**입니다. ...
```

## 8. 데이터 소스 위계

| 데이터 종류 | 1차 출처 | 라이선스 | 신뢰도 |
|------------|---------|---------|--------|
| 자형 (글자) | Unicode + KANJIDIC2 | — | 절대 신뢰 |
| 읽기 (음·훈) | KANJIDIC2 | CC BY-SA | 절대 신뢰 |
| 획수·부수·JLPT | KANJIDIC2 | CC BY-SA | 절대 신뢰 |
| 부품 분해 | KRADFILE + 검수 | CC BY-SA | 거의 신뢰 |
| 자형 변천 글리프 | GlyphWiki / 說文解字 archive | CC BY-SA | 출처 명시 |
| 어원 서술 | 說文解字 + Wiktionary + 본인 정리 | 혼합 | "해석"으로 표시 |
| 한국 한자음·뜻 | 표준국어대사전·한국한자어사전 등 | 공공 | 절대 신뢰 |

콘텐츠 정책: **하이브리드 + 출처 표기 + 사용자 제보**. 어원 서술은 학자마다 견해가 다르므로 항상 *해석*임을 명시.

## 9. 검색 아키텍처

### 진입 경로 (5가지 동시 지원)

| 입력 | 예 | 매칭 대상 |
|------|----|-----|
| 한자 직접 | `学` | 정확 일치 |
| 한국 한자음 | `학` | `korean.reading` |
| 한국어 뜻 키워드 | `배우다` | `meanings` |
| 일본어 음·훈 (가나) | `まなぶ`, `ガク` | `readings.on`, `readings.kun` |
| 일본어 발음 (로마자) | `manabu`, `gaku` | 가나의 로마자 변환 |

### 구현

**빌드 시 인덱스 생성** (Rust 빌드 바이너리):

```
content/kanji/*.md  →  dist/data/search-index.json
                        {
                          "by_kanji":    { ... },
                          "by_kr_sound": { ... },
                          "by_meaning":  { ... },
                          "by_on":       { ... },
                          "by_kun":      { ... },
                          "by_romaji":   { ... }
                        }
```

**클라이언트사이드 검색** (Dioxus + WASM):
- 검색 모달 첫 오픈 시 인덱스 lazy fetch (예상 < 200KB gzip @ 500자)
- 메모리 캐시 → 이후 검색 즉시
- 정확 일치 → 접두 일치 → 부분 일치 순 우선순위
- 한국어 입력은 자모 normalize 후 매칭

**확장 시 (2000자+) 고려사항**: 인덱스 청크 분할 (한자음 첫 자모별 등). 후순위.

## 10. 콘텐츠 파이프라인 (MVP 단계)

### 흐름

```
1. 본인이 다룰 한자 선택 (N5 핵심에서 50자)
2. AI 프롬프트 템플릿으로 초안 생성:
   [입력: 한자 + KANJIDIC 데이터 + 說文解字 발췌]
   [출력: YAML frontmatter + 어원 서술 마크다운 초안]
3. 본인이 직접 검수·수정 → commit
4. GitHub Action 트리거:
   - `build-data` Rust 바이너리 실행 (frontmatter 파싱 + 검증 + 인덱스 생성)
   - Dioxus 빌드 (WASM)
   - GitHub Pages 자동 배포
```

### 빌드 검증 (`build-data` 바이너리)
- 모든 한자 파일 frontmatter 파싱 → 타입 검증
- 필수 필드 존재 확인
- 참조 무결성 검증 (링크된 한자·부수가 실제 존재하나)
- 검색 인덱스 생성
- 역인덱스 생성 (부수 → 한자, 단어 → 한자)
- **검증 실패 시 빌드 중단** → 잘못된 데이터로 배포되는 일 없음

### Phase 2 (MVP 외)
- KANJIDIC2/KRADFILE에서 대량 시드 데이터 자동 임포트
- AI 초안 생성 → 본인 검수 워크플로 자동화

## 11. 피드백 백엔드 — Cloudflare Worker (Rust)

### 왜 별도 백엔드가 필요한가
GitHub Pages는 정적이라 API 토큰을 안전하게 다룰 수 없음. 비-개발자 사용자가 GitHub 가입 없이도 제보할 수 있게 하려면 작은 프록시가 필요.

### 엔드포인트

```
POST https://kanji-feedback.<your>.workers.dev/feedback
Content-Type: application/json

{
  "kanji": "学",
  "current_explanation_hash": "abc123",
  "suggestion": "...",
  "contact": "optional@email",
  "turnstile_token": "..."
}
```

### Worker 내부 로직 (Rust, `workers-rs`)

1. CORS 헤더 — GitHub Pages 도메인만 화이트리스트
2. Turnstile 토큰 검증 (Cloudflare 무료 캡차)
3. Rate limit (Workers KV: IP당 5건/시간)
4. 본문 validation (honeypot, 최소·최대 길이)
5. GitHub Issues API 호출:
   ```
   POST /repos/<owner>/<repo>/issues
   Authorization: Bearer <FINE_GRAINED_PAT>
   Body: {
     title: "[제보] 学 — <앞 40자>",
     body: <포맷된 마크다운>,
     labels: ["feedback", "from-web", "kanji:学"]
   }
   ```
6. 성공 시 200 + 생성된 Issue 번호 반환

### 보안
- 토큰: GitHub fine-grained PAT, `Issues:write` 전용, 해당 레포만
- Worker secret으로 저장, 클라이언트엔 절대 노출 X
- Turnstile + Rate limit + Honeypot으로 봇·남용 방지

### 비용 (모두 무료 티어 내)
- Cloudflare Workers: 10만 요청/일 (실제 사용량의 1만 배 이상)
- Cloudflare Turnstile / KV: 무료
- GitHub API: 5000/시간 (충분)

## 12. 모바일 / PWA 전략

### 반응형 우선
- 폰 ≤ 480px, 태블릿 ≤ 1024px, 데스크톱 그 이상
- CSS는 모바일 first
- 터치 영역 최소 44×44px

### PWA
- `manifest.json` + 아이콘 세트 → "홈 화면에 추가"
- 서비스 워커 (JS 표준 — Rust로 작성 불가) → 본 한자 페이지·검색 인덱스 캐시
- 오프라인 동작 (캐시된 한자 한정)
- 첫 진입 최적화: 랜딩 + 인기 한자 SSG prerender → WASM 로딩 전 HTML 보임

### 폰트
- 신자체 한자: Noto Sans JP 웹폰트 (서브셋팅, `font-display: swap`)
- 자형 변천 옛 글리프: **SVG 임베드** (시스템 폰트로 해결 불가) — GlyphWiki·說文解字 archive 활용
- 한글: 시스템 폰트 우선 (`-apple-system`, `Pretendard` 등)

### 네이티브는 MVP 외
Dioxus 모바일 타겟은 베타 — MVP에서는 PWA로 만족. 안정화 후 같은 코드베이스로 네이티브 빌드 추가 옵션.

## 13. 기술 스택 요약 (Full Rust)

| 계층 | 도구 | 언어 |
|------|------|------|
| 프론트엔드 UI / 로직 | Dioxus | Rust → WASM |
| 피드백 백엔드 (프록시) | Cloudflare Workers + `workers-rs` | Rust |
| 빌드 시 데이터 파이프라인 | 자체 Rust 바이너리 (`build-data`) | Rust |
| 단위·통합 테스트 | `cargo test` | Rust |
| 패키지·빌드 | Cargo, Dioxus CLI, Trunk | Rust 생태계 |
| 스타일링 | CSS (Tailwind 또는 일반 CSS — 미정) | CSS |
| 콘텐츠 | Markdown + YAML frontmatter | 데이터 |
| CI/CD | GitHub Actions | YAML |
| 호스팅 (정적) | GitHub Pages | — |
| 호스팅 (Worker) | Cloudflare Workers | — |
| 피드백 저장소 | GitHub Issues | — |
| 캡차 | Cloudflare Turnstile | — |

**스타일링 결정 (미정)**: (a) 일반 CSS / (b) Tailwind / (c) `stylist`(Rust-side CSS-in-Rust). 구현 계획 단계에서 결정.

## 14. MVP 범위

### ✅ 포함
- 콘텐츠 **~50자** (N5 핵심 + 어원 풍부한 한자 선별)
- 한자 페이지 (어원 스토리 + 부품 분해 통합)
- 검색 5가지 진입 경로
- `/browse` 전체 한자 둘러보기 (JLPT/획수/부수 필터)
- `/radical/{부수}` 부수 페이지
- `/radicals` 부수 인덱스
- 피드백 폼 → Cloudflare Worker → GitHub Issue 자동 생성
- PWA (설치·오프라인 캐시)
- `/about` 프로젝트 소개
- 친절한 404

### 🟡 간소화 포함
- **자형 변천**: 50자 모두 5단계 글리프를 다 채우진 못함 → 있는 것만 표시, 없으면 생략
- **부수 페이지 어원 서술**: 핵심 부수 ~20개만 깊이, 나머지는 골격 (한자 목록 + 의미)
- **단어 출현**: 한 한자당 3~5개. 형태소 분석 링크는 phase 2 placeholder

### ❌ 제외 (YAGNI)
3장 비-목표 절 참조.

## 15. 리스크 & 완화

| 리스크 | 영향 | 완화 |
|--------|------|------|
| WASM 번들 크기 | 모바일 첫 진입 느림 | 코드 스플리팅, SSG prerender, PWA 캐시 |
| 자형 변천 글리프 부재 | 옛 자형 못 보여줌 | SVG 임베드, 없으면 단계 생략 |
| 어원 정확성 비판 | "틀렸다" 클레임 위험 | "해석" 라벨, 출처, 다중 가설, 피드백 환영 |
| 콘텐츠 50자가 적게 느껴짐 | 사용자 매력 부족 | 깊이로 승부 + 큐레이션 + 매주 추가 |
| Dioxus 신생함 | API 변경·버그 위험 | 0.6+ 안정. 핀 고정. 최악의 경우 Leptos 마이그레이션 (WASM 호환) |
| 모바일 한자 입력 어려움 | 검색 마찰 | 5가지 진입 경로가 정확히 이 문제 해결 |
| Rust+WASM 빌드 시간 | 배포 느림 | `Swatinem/rust-cache`, target 캐싱 |
| Cloudflare Worker 한도 | 폭증 시 위험 | MVP 트래픽은 한도의 0.01% 예상. 모니터링만 |
| 피드백 스팸 | Issue 더미 | Turnstile + Rate limit + 모더레이션 |

## 16. 성공 기준

### 기술
- [ ] 50자 한자 페이지 모두 완성 (어원 서술 + 부품 + 자형 변천 일부)
- [ ] 5가지 검색 모드 모두 동작
- [ ] 첫 진입 LCP < 3초 (모바일 4G 기준)
- [ ] PWA 설치·오프라인 동작 검증
- [ ] 피드백 폼 → GitHub Issue 자동 생성 검증

### 콘텐츠
- [ ] 모든 한자 페이지에 출처 명시
- [ ] 어원 서술 검수 완료 (본인 + 가능하면 외부 1명 리뷰)

### 운영
- [ ] GitHub Issues 제보를 PR로 반영하는 사이클 1회 이상 실증
- [ ] 외부 사용자 5명 이상 시범 사용 + 피드백 제출 흐름 완주

## 17. Phase 2 (MVP 외 — 참고용)

- KANJIDIC2/KRADFILE 대량 임포트 파이프라인
- 부수 페이지 어원 서술 깊이 채우기
- 형태소 분석 모듈 (다음 하위 프로젝트) 연동
- 다크 모드
- 즐겨찾기 클라우드 동기화 (Cloudflare D1 검토)
- 검색 typeahead / 자동완성
- Dioxus mobile 네이티브 빌드
- 가나 모듈 / 문법 모듈 (별도 하위 프로젝트)
- 다국어 UI (일본어·영어)

---

## 부록 A — 폴더 구조 (예상)

```
/
├── Cargo.toml                  # 워크스페이스
├── crates/
│   ├── web/                    # Dioxus 앱
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── build-data/             # 빌드 바이너리
│   │   ├── Cargo.toml
│   │   └── src/
│   └── feedback-worker/        # Cloudflare Worker
│       ├── Cargo.toml
│       ├── wrangler.toml
│       └── src/
├── content/
│   ├── kanji/                  # 한자 1자 = 1 .md 파일
│   └── radicals/               # 부수 정보
├── public/                     # 정적 자산 (PWA manifest, 아이콘, 폰트)
├── .github/workflows/          # CI/CD
└── docs/                       # 설계 문서 (이 문서 포함)
```

## 부록 B — 합의된 결정 일람

| 결정 | 값 |
|------|----|
| 영역 | 한자 어원 학습 모듈 (종합 일본어 서비스의 1번째 하위 프로젝트) |
| 타겟 | 한국인 일본어 학습자 (한자 지식 약함 전제) |
| 진입 레벨 | 분리 없음, 전 레벨 관통 |
| 학습 형태 | 사전형 탐색 |
| 한자 페이지 | 어원 스토리 + 부품 분해 통합 |
| 콘텐츠 정책 | 하이브리드 + 출처 + 사용자 제보 |
| 프로젝트 성격 | 개인 / 포트폴리오 |
| URL | `/kanji/{한자}`, `/radical/{부수}` |
| 스택 | Dioxus + Cloudflare Workers (Rust) + GitHub Pages + GitHub Issues |
| 데이터 | YAML frontmatter + Markdown, 1한자=1파일 |
| 검색 | 클라이언트사이드, 5진입 |
| 피드백 | 인앱 폼 → Worker → GitHub Issue |
| 모바일 | 반응형 + PWA (네이티브는 후속) |
| MVP 콘텐츠 | ~50자 |

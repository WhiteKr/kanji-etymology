/*
 * 한자 어원 사전 — 서비스 워커 (M8 PWA)
 *
 * 배치: 이 파일은 배포 산출물의 **루트**(= base path 루트, /kanji-etymology/sw.js)에
 * 복사된다 (CI 워크플로 참조). 서비스 워커의 scope는 파일 위치 이하로 제한되므로
 * dx asset 시스템(/assets/ 하위)이 아니라 루트에 두어야 사이트 전체를 제어할 수 있다.
 * base path는 SW 자신의 위치에서 유도하므로(BASE) 경로가 바뀌어도 수정이 필요 없다.
 *
 * 캐시 전략:
 * - 앱 셸(index.html + 해시 이름 js/wasm/css): install 시 프리캐시.
 *   해시 파일명은 빌드마다 바뀌므로 정적 목록 대신 본문에서 추출한다 —
 *   index.html에 js 경로가, js 본문에 wasm 경로가, wasm 본문(문자열 섹션)에
 *   css 경로가 들어 있어 3단계로 따라가며 수집한다 (precacheShell).
 * - /assets/data/ JSON(한자·부수·인덱스): stale-while-revalidate —
 *   캐시를 즉시 응답하고 백그라운드에서 갱신. 방문한 페이지는 오프라인 재방문 가능.
 * - 그 외 /assets/ 정적 파일: cache-first (내용 해시 파일명 = 불변).
 * - SPA 내비게이션: network-first, 오프라인이면 캐시된 앱 셸로 폴백.
 * - 크로스 오리진(구글 폰트 등)은 건드리지 않는다 (브라우저 기본 처리).
 *
 * 캐시 무효화: CI가 __BUILD_VERSION__ 을 커밋 SHA로 치환한다.
 * 버전이 바뀌면 activate 단계에서 이전 버전 캐시를 전부 삭제.
 */

const CACHE_VERSION = "__BUILD_VERSION__";
const CACHE_NAME = `kanji-etymology-${CACHE_VERSION}`;

// base path 루트 — SW 파일 위치에서 유도 ("/kanji-etymology" 또는 루트 배포면 "")
const BASE = self.location.pathname.replace(/\/sw\.js$/, "");

/** 본문 텍스트에서 해시 이름 정적 애셋 경로를 추출해 base 접두어를 붙인다. */
function extractAssetPaths(text) {
  const found = new Set();
  // 1) 전체 경로 형태 — index.html(<script src>)과 js 본문(wasm 로더)에 등장
  const fullRe = /\/assets\/[\w.-]+\.(?:js|wasm|css)/g;
  let m;
  while ((m = fullRe.exec(text)) !== null) {
    found.add(BASE + m[0]);
  }
  // 2) 파일명만 있는 형태 — wasm 바이너리의 asset 메타데이터(bundled_path)는
  //    "main-dxh….css"처럼 디렉터리 없이 저장된다. dx 해시 인픽스("-dxh"+hex)가
  //    확실한 표식이므로 이것만 매칭한다 (런타임에 /assets/ 아래로 resolve됨).
  const bareRe = /[\w-]+-dxh[0-9a-f]+\.(?:js|wasm|css)/g;
  while ((m = bareRe.exec(text)) !== null) {
    found.add(`${BASE}/assets/${m[0]}`);
  }
  return [...found];
}

/** 앱 셸 프리캐시 — index.html에서 출발해 js → wasm → css 경로를 따라간다. */
async function precacheShell(cache) {
  // "./" = base path 루트의 index.html (SW 위치 기준 상대 경로)
  const shellResp = await fetch("./");
  if (!shellResp.ok) return;
  await cache.put("./", shellResp.clone());

  const seen = new Set();
  const queue = extractAssetPaths(await shellResp.text());
  while (queue.length > 0) {
    const path = queue.shift();
    if (seen.has(path)) continue;
    seen.add(path);
    try {
      const resp = await fetch(path);
      if (!resp.ok) continue;
      await cache.put(path, resp.clone());
      // js/wasm 본문에 다음 단계 애셋 경로가 문자열로 들어 있다
      if (/\.(?:js|wasm)$/.test(path)) {
        queue.push(...extractAssetPaths(await resp.text()));
      }
    } catch (e) {
      // 프리캐시 실패는 치명적이지 않다 — 런타임 캐시가 이후 방문에서 채운다
      console.warn("[sw] 프리캐시 실패:", path, e);
    }
  }
}

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(CACHE_NAME);
      await precacheShell(cache);
      await self.skipWaiting();
    })()
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      // 버전이 다른 이전 캐시 제거 (버전 문자열 기반 무효화)
      const keys = await caches.keys();
      await Promise.all(
        keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k))
      );
      await self.clients.claim();
    })()
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);
  // 크로스 오리진(웹폰트 등)은 브라우저 기본 처리에 맡긴다
  if (url.origin !== self.location.origin) return;

  // SPA 내비게이션: network-first → 오프라인이면 캐시된 앱 셸
  // (라우팅은 어차피 클라이언트에서 처리하므로 어떤 경로든 셸이면 충분)
  if (req.mode === "navigate") {
    event.respondWith(
      (async () => {
        try {
          const resp = await fetch(req);
          if (resp.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put("./", resp.clone());
          }
          return resp;
        } catch {
          const cached = await caches.match("./");
          if (cached) return cached;
          throw new Error("오프라인 상태이며 캐시된 앱 셸이 없습니다");
        }
      })()
    );
    return;
  }

  // 데이터 JSON: stale-while-revalidate
  if (url.pathname.includes("/assets/data/")) {
    event.respondWith(
      (async () => {
        const cache = await caches.open(CACHE_NAME);
        const cached = await cache.match(req);
        const network = fetch(req)
          .then((resp) => {
            if (resp.ok) cache.put(req, resp.clone());
            return resp;
          })
          .catch(() => cached);
        return cached || network;
      })()
    );
    return;
  }

  // 그 외 정적 애셋(해시 파일명 js/wasm/css, PWA 아이콘 등): cache-first
  if (url.pathname.includes("/assets/")) {
    event.respondWith(
      (async () => {
        const cache = await caches.open(CACHE_NAME);
        const cached = await cache.match(req);
        if (cached) return cached;
        const resp = await fetch(req);
        if (resp.ok) cache.put(req, resp.clone());
        return resp;
      })()
    );
  }
});

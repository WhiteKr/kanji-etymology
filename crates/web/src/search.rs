//! 검색 매칭 로직 (M5) — 순수 함수 모듈.
//!
//! 설계 문서 9장(검색 아키텍처)의 5가지 진입 경로를 모두 처리한다:
//! 한자 직접 / 한국 한자음 / 한국어 뜻 / 일본어 음·훈(가나) / 로마자.
//!
//! dioxus·브라우저 API에 의존하지 않으므로 wasm이 아닌 호스트에서도
//! `cargo test -p kanji-web`로 검증할 수 있다.

use std::collections::HashMap;

use serde::Deserialize;

/// `search-index.json`의 구조 (build-data의 `SearchIndex` 출력과 대응).
/// `by_kanji`만 값이 문자열이고 나머지는 한자 배열이다.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct SearchIndex {
    /// 한자 → 한자 (등재 여부 조회용)
    pub by_kanji: HashMap<String, String>,
    /// 한국 한자음("학") → 한자들
    pub by_kr_sound: HashMap<String, Vec<String>>,
    /// 한국어 뜻 키워드("배우다") → 한자들
    pub by_meaning: HashMap<String, Vec<String>>,
    /// 음독 가타카나("ガク") → 한자들
    pub by_on: HashMap<String, Vec<String>>,
    /// 훈독 히라가나("まなぶ") → 한자들
    pub by_kun: HashMap<String, Vec<String>>,
    /// 헵번 로마자("gaku") → 한자들
    pub by_romaji: HashMap<String, Vec<String>>,
}

/// 매칭 우선순위. 낮을수록(위 variant일수록) 우선.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatchRank {
    /// 정확 일치
    Exact,
    /// 접두 일치
    Prefix,
    /// 부분(포함) 일치
    Partial,
}

/// 검색 결과 한 건 — 한자와 그 한자가 얻은 최고 우선순위.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub kanji: String,
    pub rank: MatchRank,
}

/// 6개 맵 전부를 대상으로 검색해서 우선순위(정확 > 접두 > 부분)로
/// 정렬된 결과를 돌려준다. 여러 맵에서 겹치는 한자는 최고 우선순위 하나로 병합.
pub fn search(index: &SearchIndex, query: &str) -> Vec<SearchHit> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }

    // 한자 → 지금까지의 최고 우선순위
    let mut best: HashMap<String, MatchRank> = HashMap::new();

    // 1) 한자 직접 — 정확 일치. 여러 글자를 붙여넣은 경우(예: "学校")에는
    //    등재된 글자 각각을 부분 일치로 건진다.
    if index.by_kanji.contains_key(q) {
        note(&mut best, q, MatchRank::Exact);
    } else if q.chars().count() > 1 {
        for ch in q.chars() {
            let s = ch.to_string();
            if index.by_kanji.contains_key(&s) {
                note(&mut best, &s, MatchRank::Partial);
            }
        }
    }

    // 2) 한국 한자음 + 3) 한국어 뜻 — 자모 분해 후 비교.
    //    "하"를 치면 "학"(ㅎㅏㄱ)이 접두 후보로 떠야 하므로 양쪽 다 자모로 푼다.
    let q_jamo = decompose_jamo(q);
    match_map(&index.by_kr_sound, &mut best, |key| {
        rank_of(&decompose_jamo(key), &q_jamo)
    });
    match_map(&index.by_meaning, &mut best, |key| {
        rank_of(&decompose_jamo(key), &q_jamo)
    });

    // 4) 음독(가타카나 키) / 5) 훈독(히라가나 키) — 입력을 양쪽 표기로
    //    변환해서 조회하므로 히라가나·가타카나 어느 쪽으로 쳐도 매칭된다.
    let q_kata = to_katakana(q);
    match_map(&index.by_on, &mut best, |key| rank_of(key, &q_kata));
    let q_hira = to_hiragana(q);
    match_map(&index.by_kun, &mut best, |key| rank_of(key, &q_hira));

    // 6) 로마자 — 소문자화 후 정확·접두 일치만 (부분 일치는 잡음이 많음).
    let q_lower = q.to_lowercase();
    if !q_lower.is_empty() && q_lower.chars().all(|c| c.is_ascii_alphabetic()) {
        match_map(&index.by_romaji, &mut best, |key| {
            match rank_of(key, &q_lower) {
                Some(MatchRank::Partial) => None,
                other => other,
            }
        });
    }

    // 우선순위 → 한자 코드포인트 순으로 결정적 정렬.
    let mut hits: Vec<SearchHit> = best
        .into_iter()
        .map(|(kanji, rank)| SearchHit { kanji, rank })
        .collect();
    hits.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.kanji.cmp(&b.kanji)));
    hits
}

/// 한자에 우선순위를 기록하되, 이미 더 높은(작은) 순위가 있으면 유지한다.
fn note(best: &mut HashMap<String, MatchRank>, kanji: &str, rank: MatchRank) {
    match best.get_mut(kanji) {
        Some(existing) => {
            if rank < *existing {
                *existing = rank;
            }
        }
        None => {
            best.insert(kanji.to_string(), rank);
        }
    }
}

/// `키 → [한자]` 맵 하나를 훑으며 `rank_fn`이 매긴 순위를 기록한다.
fn match_map<F>(map: &HashMap<String, Vec<String>>, best: &mut HashMap<String, MatchRank>, rank_fn: F)
where
    F: Fn(&str) -> Option<MatchRank>,
{
    for (key, kanjis) in map {
        if let Some(rank) = rank_fn(key) {
            for kanji in kanjis {
                note(best, kanji, rank);
            }
        }
    }
}

/// 정규화가 끝난 키와 쿼리를 비교해 순위를 매긴다.
fn rank_of(key: &str, query: &str) -> Option<MatchRank> {
    if query.is_empty() {
        return None;
    }
    if key == query {
        Some(MatchRank::Exact)
    } else if key.starts_with(query) {
        Some(MatchRank::Prefix)
    } else if key.contains(query) {
        Some(MatchRank::Partial)
    } else {
        None
    }
}

// ── 가나 상호 변환 ──────────────────────────────────────────────

/// 가타카나(U+30A1..=U+30F6) → 히라가나. 그 외 문자는 그대로.
pub fn to_hiragana(s: &str) -> String {
    s.chars()
        .map(|ch| match ch as u32 {
            code @ 0x30A1..=0x30F6 => char::from_u32(code - 0x60).unwrap_or(ch),
            _ => ch,
        })
        .collect()
}

/// 히라가나(U+3041..=U+3096) → 가타카나. 그 외 문자는 그대로.
pub fn to_katakana(s: &str) -> String {
    s.chars()
        .map(|ch| match ch as u32 {
            code @ 0x3041..=0x3096 => char::from_u32(code + 0x60).unwrap_or(ch),
            _ => ch,
        })
        .collect()
}

// ── 한국어 자모 분해 ────────────────────────────────────────────

/// 초성 19자 (유니코드 산술 인덱스 순).
const CHOSEONG: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ',
    'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/// 중성 21자 — 겹모음은 낱자모로 풀어 둔다 ("호"가 "화"의 접두가 되도록).
const JUNGSEONG: [&str; 21] = [
    "ㅏ", "ㅐ", "ㅑ", "ㅒ", "ㅓ", "ㅔ", "ㅕ", "ㅖ", "ㅗ", "ㅗㅏ", "ㅗㅐ", "ㅗㅣ", "ㅛ", "ㅜ",
    "ㅜㅓ", "ㅜㅔ", "ㅜㅣ", "ㅠ", "ㅡ", "ㅡㅣ", "ㅣ",
];

/// 종성 28자 (첫 칸은 받침 없음) — 겹받침도 낱자모로 풀어 둔다.
const JONGSEONG: [&str; 28] = [
    "", "ㄱ", "ㄲ", "ㄱㅅ", "ㄴ", "ㄴㅈ", "ㄴㅎ", "ㄷ", "ㄹ", "ㄹㄱ", "ㄹㅁ", "ㄹㅂ", "ㄹㅅ",
    "ㄹㅌ", "ㄹㅍ", "ㄹㅎ", "ㅁ", "ㅂ", "ㅂㅅ", "ㅅ", "ㅆ", "ㅇ", "ㅈ", "ㅊ", "ㅋ", "ㅌ", "ㅍ",
    "ㅎ",
];

/// 문자열의 한글 음절을 호환 자모(초·중·종성)로 분해한다.
/// 한글이 아닌 문자는 그대로 통과. 예: "학" → "ㅎㅏㄱ", "하" → "ㅎㅏ".
pub fn decompose_jamo(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        let code = ch as u32;
        if (0xAC00..=0xD7A3).contains(&code) {
            // 완성형 음절 — 유니코드 산술로 분해
            let idx = code - 0xAC00;
            out.push(CHOSEONG[(idx / 588) as usize]);
            out.push_str(JUNGSEONG[((idx % 588) / 28) as usize]);
            out.push_str(JONGSEONG[(idx % 28) as usize]);
        } else if let Some(expanded) = expand_compat_jamo(ch) {
            // 호환 자모로 직접 입력된 겹자모도 낱자모로 풀어 준다
            out.push_str(expanded);
        } else {
            // 한글 아님 — 그대로 통과
            out.push(ch);
        }
    }
    out
}

/// 호환 자모 겹자모 → 낱자모 전개. 해당 없으면 `None`.
fn expand_compat_jamo(ch: char) -> Option<&'static str> {
    let expanded = match ch {
        'ㅘ' => "ㅗㅏ",
        'ㅙ' => "ㅗㅐ",
        'ㅚ' => "ㅗㅣ",
        'ㅝ' => "ㅜㅓ",
        'ㅞ' => "ㅜㅔ",
        'ㅟ' => "ㅜㅣ",
        'ㅢ' => "ㅡㅣ",
        'ㄳ' => "ㄱㅅ",
        'ㄵ' => "ㄴㅈ",
        'ㄶ' => "ㄴㅎ",
        'ㄺ' => "ㄹㄱ",
        'ㄻ' => "ㄹㅁ",
        'ㄼ' => "ㄹㅂ",
        'ㄽ' => "ㄹㅅ",
        'ㄾ' => "ㄹㅌ",
        'ㄿ' => "ㄹㅍ",
        'ㅀ' => "ㄹㅎ",
        'ㅄ' => "ㅂㅅ",
        _ => return None,
    };
    Some(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 学 하나를 가진 미니 인덱스 (실데이터 search-index.json과 같은 형태).
    fn fixture() -> SearchIndex {
        let mut idx = SearchIndex::default();
        idx.by_kanji.insert("学".into(), "学".into());
        idx.by_kanji.insert("山".into(), "山".into());
        idx.by_kr_sound.insert("학".into(), vec!["学".into()]);
        idx.by_kr_sound.insert("산".into(), vec!["山".into()]);
        idx.by_meaning.insert("배우다".into(), vec!["学".into()]);
        idx.by_meaning.insert("학문".into(), vec!["学".into()]);
        idx.by_meaning.insert("산".into(), vec!["山".into()]);
        idx.by_on.insert("ガク".into(), vec!["学".into()]);
        idx.by_kun.insert("まなぶ".into(), vec!["学".into()]);
        idx.by_kun.insert("やま".into(), vec!["山".into()]);
        idx.by_romaji.insert("gaku".into(), vec!["学".into()]);
        idx.by_romaji.insert("manabu".into(), vec!["学".into()]);
        idx
    }

    fn kanjis(hits: &[SearchHit]) -> Vec<&str> {
        hits.iter().map(|h| h.kanji.as_str()).collect()
    }

    // ── 5가지 진입 경로 모두 学 도달 (M5 완료 기준) ──────────

    #[test]
    fn finds_by_kanji_direct() {
        let hits = search(&fixture(), "学");
        assert_eq!(hits[0].kanji, "学");
        assert_eq!(hits[0].rank, MatchRank::Exact);
    }

    #[test]
    fn finds_by_korean_sound() {
        let hits = search(&fixture(), "학");
        assert_eq!(hits[0].kanji, "学");
        assert_eq!(hits[0].rank, MatchRank::Exact);
    }

    #[test]
    fn finds_by_korean_meaning() {
        let hits = search(&fixture(), "배우다");
        assert_eq!(hits[0].kanji, "学");
        assert_eq!(hits[0].rank, MatchRank::Exact);
    }

    #[test]
    fn finds_by_kun_hiragana_and_on_katakana() {
        let hits = search(&fixture(), "まなぶ");
        assert_eq!(hits[0].kanji, "学");
        let hits = search(&fixture(), "ガク");
        assert_eq!(hits[0].kanji, "学");
    }

    #[test]
    fn kana_cross_script_lookup() {
        // 가타카나로 훈독을 쳐도, 히라가나로 음독을 쳐도 찾아야 한다
        let hits = search(&fixture(), "マナブ");
        assert_eq!(hits[0].kanji, "学");
        let hits = search(&fixture(), "がく");
        assert_eq!(hits[0].kanji, "学");
    }

    #[test]
    fn finds_by_romaji() {
        let hits = search(&fixture(), "manabu");
        assert_eq!(hits[0].kanji, "学");
        let hits = search(&fixture(), "gaku");
        assert_eq!(hits[0].kanji, "学");
        // 대문자·공백 섞여도 정규화
        let hits = search(&fixture(), "  GAKU ");
        assert_eq!(hits[0].kanji, "学");
    }

    // ── 자모 접두 매칭 ────────────────────────────────────────

    #[test]
    fn jamo_prefix_matches_incomplete_syllable() {
        // "하"(ㅎㅏ)는 "학"(ㅎㅏㄱ)의 자모 접두
        let hits = search(&fixture(), "하");
        assert!(hits.iter().any(|h| h.kanji == "学" && h.rank == MatchRank::Prefix));
    }

    #[test]
    fn jamo_single_consonant_prefix() {
        // 초성 "ㅎ"만 쳐도 "학"이 접두 후보
        let hits = search(&fixture(), "ㅎ");
        assert!(hits.iter().any(|h| h.kanji == "学" && h.rank == MatchRank::Prefix));
    }

    #[test]
    fn decompose_jamo_arithmetic() {
        assert_eq!(decompose_jamo("학"), "ㅎㅏㄱ");
        assert_eq!(decompose_jamo("하"), "ㅎㅏ");
        assert_eq!(decompose_jamo("화"), "ㅎㅗㅏ"); // 겹모음 전개
        assert_eq!(decompose_jamo("값"), "ㄱㅏㅂㅅ"); // 겹받침 전개
        assert_eq!(decompose_jamo("abc学"), "abc学"); // 비한글 통과
    }

    // ── 우선순위 정렬·병합 ───────────────────────────────────

    #[test]
    fn priority_exact_before_prefix_before_partial() {
        let mut idx = SearchIndex::default();
        // "산" 정확 / "산더미" 접두 / "화산" 부분 — 서로 다른 한자에 배정
        idx.by_meaning.insert("산".into(), vec!["山".into()]);
        idx.by_meaning.insert("산더미".into(), vec!["学".into()]);
        idx.by_meaning.insert("화산".into(), vec!["日".into()]);
        let hits = search(&idx, "산");
        assert_eq!(kanjis(&hits), vec!["山", "学", "日"]);
        assert_eq!(
            hits.iter().map(|h| h.rank).collect::<Vec<_>>(),
            vec![MatchRank::Exact, MatchRank::Prefix, MatchRank::Partial]
        );
    }

    #[test]
    fn duplicate_kanji_merged_with_best_rank() {
        let mut idx = SearchIndex::default();
        // 같은 한자가 접두(산더미)와 정확(산) 양쪽에서 나오면 정확 하나로 병합
        idx.by_kr_sound.insert("산".into(), vec!["山".into()]);
        idx.by_meaning.insert("산더미".into(), vec!["山".into()]);
        let hits = search(&idx, "산");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kanji, "山");
        assert_eq!(hits[0].rank, MatchRank::Exact);
    }

    #[test]
    fn multi_char_paste_finds_registered_kanji() {
        // "学校"를 붙여넣으면 등재된 学만 부분 일치로 건진다
        let hits = search(&fixture(), "学校");
        assert!(hits.iter().any(|h| h.kanji == "学" && h.rank == MatchRank::Partial));
    }

    #[test]
    fn empty_and_whitespace_queries_return_nothing() {
        assert!(search(&fixture(), "").is_empty());
        assert!(search(&fixture(), "   ").is_empty());
    }

    #[test]
    fn romaji_prefix_but_not_partial() {
        // "man"은 "manabu"의 접두 → 매칭. "abu"는 부분 → 제외(잡음 방지)
        let hits = search(&fixture(), "man");
        assert!(hits.iter().any(|h| h.kanji == "学" && h.rank == MatchRank::Prefix));
        let hits = search(&fixture(), "abu");
        assert!(hits.is_empty());
    }

    #[test]
    fn kana_conversion_roundtrip() {
        assert_eq!(to_hiragana("ガク"), "がく");
        assert_eq!(to_katakana("まなぶ"), "マナブ");
        assert_eq!(to_hiragana("学ぶ"), "学ぶ"); // 비가나 통과
    }
}

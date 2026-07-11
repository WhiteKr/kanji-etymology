//! 검색 인덱스 및 역인덱스 생성.
//!
//! 출력 순서가 재현 가능하도록(git diff 친화적) `HashMap` 대신
//! `BTreeMap`을 사용한다.

use std::collections::BTreeMap;

use kanji_schema::{KanjiEntry, RadicalEntry};
use serde::Serialize;

use crate::kana::kana_to_romaji;

/// `search-index.json`의 구조. 설계 문서 9장 참조.
#[derive(Debug, Default, Serialize)]
pub struct SearchIndex {
    pub by_kanji: BTreeMap<String, String>,
    pub by_kr_sound: BTreeMap<String, Vec<String>>,
    pub by_meaning: BTreeMap<String, Vec<String>>,
    pub by_on: BTreeMap<String, Vec<String>>,
    pub by_kun: BTreeMap<String, Vec<String>>,
    pub by_romaji: BTreeMap<String, Vec<String>>,
}

fn push_unique(map: &mut BTreeMap<String, Vec<String>>, key: String, value: &str) {
    if key.is_empty() {
        return;
    }
    let entry = map.entry(key).or_default();
    if !entry.iter().any(|v| v == value) {
        entry.push(value.to_string());
    }
}

/// 훈독 `reading` 필드에서 어간-오쿠리가나 구분자 `-`를 제거해 순수 가나
/// 읽기를 얻는다 (예: `"まな-ぶ"` → `"まなぶ"`). 이 값은 정확히 오쿠리가나가
/// 붙은 `form`(예: `"学ぶ"`)이 실제로 읽히는 발음과 같다.
pub fn normalize_kun_reading(reading: &str) -> String {
    reading.replace('-', "")
}

/// 한국어 뜻 문자열에서 검색 키워드 목록을 만든다. 원문 전체와, 쉼표·공백으로
/// 쪼갠 각 키워드를 모두 포함한다 (예: `"학문, 공부"` → `["학문, 공부", "학문", "공부"]`).
pub fn meaning_keywords(meaning: &str) -> Vec<String> {
    let trimmed = meaning.trim();
    let mut keys = vec![trimmed.to_string()];
    for part in trimmed.split(|c: char| c == ',' || c.is_whitespace()) {
        let p = part.trim();
        if !p.is_empty() && p != trimmed {
            keys.push(p.to_string());
        }
    }
    keys
}

/// 한자 목록으로부터 5가지 진입 경로용 검색 인덱스를 만든다.
pub fn build_search_index(entries: &[KanjiEntry]) -> SearchIndex {
    let mut idx = SearchIndex::default();
    for entry in entries {
        let c = entry.character.as_str();
        idx.by_kanji.insert(c.to_string(), c.to_string());

        push_unique(&mut idx.by_kr_sound, entry.korean.reading.clone(), c);

        for meaning in &entry.meanings {
            for key in meaning_keywords(meaning) {
                push_unique(&mut idx.by_meaning, key, c);
            }
        }

        for on in &entry.readings.on {
            push_unique(&mut idx.by_on, on.clone(), c);
            push_unique(&mut idx.by_romaji, kana_to_romaji(on), c);
        }

        for kun in &entry.readings.kun {
            let normalized = normalize_kun_reading(&kun.reading);
            push_unique(&mut idx.by_kun, normalized.clone(), c);
            push_unique(&mut idx.by_romaji, kana_to_romaji(&normalized), c);
        }
    }
    idx
}

/// 부품(부수·부품 문자) → 그 부품을 구성 요소로 갖는 한자 목록 (`by-radical.json`).
pub fn build_by_radical_index(entries: &[KanjiEntry]) -> BTreeMap<String, Vec<String>> {
    let mut idx = BTreeMap::new();
    for entry in entries {
        for component in &entry.components {
            push_unique(&mut idx, component.form.clone(), &entry.character);
        }
    }
    idx
}

/// 단어 → 그 단어가 실린 한자 목록 (`by-word.json`).
pub fn build_by_word_index(entries: &[KanjiEntry]) -> BTreeMap<String, Vec<String>> {
    let mut idx = BTreeMap::new();
    for entry in entries {
        for word in &entry.words {
            push_unique(&mut idx, word.word.clone(), &entry.character);
        }
    }
    idx
}

/// `kanji-list.json`의 항목 하나 (browse용 요약).
#[derive(Debug, Serialize)]
pub struct KanjiSummaryItem {
    pub character: String,
    pub korean_reading: String,
    pub meanings: Vec<String>,
    pub jlpt: Option<String>,
    pub stroke_count: Option<u32>,
    pub grade: Option<u8>,
}

/// `kanji-list.json` 배열을 만든다.
pub fn build_kanji_list(entries: &[KanjiEntry]) -> Vec<KanjiSummaryItem> {
    entries
        .iter()
        .map(|e| KanjiSummaryItem {
            character: e.character.clone(),
            korean_reading: e.korean.reading.clone(),
            meanings: e.meanings.clone(),
            jlpt: e.jlpt.clone(),
            stroke_count: e.stroke_count,
            grade: e.grade,
        })
        .collect()
}

/// `radicals-list.json`의 항목 하나 (부수 인덱스 페이지용 요약).
#[derive(Debug, Serialize)]
pub struct RadicalSummaryItem {
    pub radical: String,
    pub name: String,
    pub meaning: String,
    pub stroke_count: u32,
    /// 이 부수를 구성 요소로 갖는 등재 한자 수 (by-radical 역인덱스 기준).
    pub kanji_count: usize,
}

/// `radicals-list.json` 배열을 만든다. 획수 → 부수 문자 순으로 정렬해
/// 출력이 결정적이고 인덱스 페이지에서 그대로 쓸 수 있게 한다.
pub fn build_radicals_list(
    radicals: &[RadicalEntry],
    by_radical: &BTreeMap<String, Vec<String>>,
) -> Vec<RadicalSummaryItem> {
    let mut items: Vec<RadicalSummaryItem> = radicals
        .iter()
        .map(|r| RadicalSummaryItem {
            radical: r.radical.clone(),
            name: r.name.clone(),
            meaning: r.meaning.clone(),
            stroke_count: r.stroke_count,
            kanji_count: by_radical.get(&r.radical).map_or(0, Vec::len),
        })
        .collect();
    items.sort_by(|a, b| {
        a.stroke_count
            .cmp(&b.stroke_count)
            .then_with(|| a.radical.cmp(&b.radical))
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_kun_reading_removes_dash() {
        assert_eq!(normalize_kun_reading("まな-ぶ"), "まなぶ");
    }

    #[test]
    fn meaning_keywords_splits_on_comma_and_space() {
        let keys = meaning_keywords("학문, 공부");
        assert_eq!(keys, vec!["학문, 공부", "학문", "공부"]);
    }

    #[test]
    fn meaning_keywords_single_word_has_one_key() {
        assert_eq!(meaning_keywords("배우다"), vec!["배우다"]);
    }

    #[test]
    fn radicals_list_counts_members_and_sorts_by_stroke() {
        let radicals = vec![
            RadicalEntry {
                radical: "木".into(),
                name: "나무목".into(),
                meaning: "나무".into(),
                stroke_count: 4,
                variants: vec![],
                last_updated: "2026-07-11".into(),
            },
            RadicalEntry {
                radical: "冖".into(),
                name: "민갓머리".into(),
                meaning: "덮다".into(),
                stroke_count: 2,
                variants: vec![],
                last_updated: "2026-07-11".into(),
            },
        ];
        let mut by_radical: BTreeMap<String, Vec<String>> = BTreeMap::new();
        by_radical.insert("冖".into(), vec!["学".into(), "字".into()]);

        let list = build_radicals_list(&radicals, &by_radical);
        // 획수 오름차순 정렬: 冖(2획) → 木(4획)
        assert_eq!(list[0].radical, "冖");
        assert_eq!(list[0].kanji_count, 2);
        // 역인덱스에 없는 부수는 0자로 집계
        assert_eq!(list[1].radical, "木");
        assert_eq!(list[1].kanji_count, 0);
    }
}

//! KANJIDIC2에서 뽑은 기계적 필드로 초안 스켈레톤 `content/kanji/{글자}.md`를
//! 만든다. 어원 서술(본문)은 사람/AI가 나중에 채우는 몫이므로, 여기서는
//! `<!-- TODO: 어원 서술 작성 -->` 마커만 남긴다.
//!
//! 스켈레톤은 `kanji-schema::KanjiEntry`에 맞는 **완전한** frontmatter를
//! 갖지만, 값을 알 수 없는 필드는 `"TODO"` placeholder로 채운다
//! (`jlpt`, `korean.meaning`, `last_updated` 등). 이 때문에 스켈레톤은
//! `build-data`의 콘텐츠 검증(`cargo run -p build-data`)을 통과하지 못한다
//! — 의도된 동작이다. TODO를 실제 값으로 채운 뒤에야 정식 콘텐츠가 된다.

use anyhow::{Context, Result};
use kanji_schema::{Confidence, KanjiEntry, Korean, KunReading, Readings, Source};

use crate::kanjidic::KanjidicChar;

/// 값을 모를 때 채워 넣는 placeholder. 검색하기 쉽도록 통일된 문자열을 쓴다.
pub const TODO: &str = "TODO";

/// KANJIDIC2 라이선스 고지 (스켈레톤 sources에 항상 포함).
pub const KANJIDIC2_LICENSE_URL: &str = "http://www.edrdg.org/wiki/index.php/KANJIDIC_Project";

/// `KanjidicChar` → 스켈레톤 `KanjiEntry` 변환.
pub fn build_skeleton_entry(k: &KanjidicChar) -> KanjiEntry {
    let korean_reading = if k.korean_h.is_empty() {
        TODO.to_string()
    } else {
        k.korean_h.join("/")
    };

    let meanings: Vec<String> = if k.meanings_en.is_empty() {
        vec![format!("(en) {TODO}")]
    } else {
        k.meanings_en.iter().map(|m| format!("(en) {m}")).collect()
    };

    KanjiEntry {
        character: k.literal.to_string(),
        variants: Vec::new(),
        jlpt: Some(TODO.to_string()),
        grade: k.grade,
        stroke_count: k.stroke_count,
        korean: Korean {
            reading: korean_reading,
            meaning: TODO.to_string(),
        },
        readings: Readings {
            on: k.on.clone(),
            kun: k
                .kun
                .iter()
                .map(|kp| KunReading {
                    reading: kp.reading.clone(),
                    form: kp.form.clone(),
                })
                .collect(),
        },
        meanings,
        components: Vec::new(),
        evolution: Vec::new(),
        related: Vec::new(),
        words: Vec::new(),
        confidence: Confidence::Interpretation,
        sources: vec![Source {
            name: "KANJIDIC2".to_string(),
            r#ref: None,
            license: Some("CC-BY-SA".to_string()),
            url: Some(KANJIDIC2_LICENSE_URL.to_string()),
            year: None,
        }],
        last_updated: TODO.to_string(),
    }
}

/// `KanjiEntry`를 `---\n<YAML>\n---\n\n<본문>` 형식의 마크다운 파일 전체
/// 텍스트로 렌더링한다. `frontmatter::split_frontmatter`가 되돌려 파싱할 수
/// 있는 형식이다 (`--skip-existing` 등에서 재사용 가능하도록).
pub fn render_skeleton_markdown(entry: &KanjiEntry) -> Result<String> {
    let mut yaml =
        serde_yaml::to_string(entry).context("스켈레톤 frontmatter YAML 직렬화에 실패했습니다")?;
    // serde_yaml 0.9는 문서 시작에 "---\n"을 자동으로 붙인다. 우리가 직접
    // 감싸는 형식과 중복되지 않도록 있으면 제거해 둔다.
    if let Some(stripped) = yaml.strip_prefix("---\n") {
        yaml = stripped.to_string();
    }
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }

    Ok(format!(
        "---\n{yaml}---\n\n<!-- TODO: 어원 서술 작성 -->\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::parse_kanji_file;
    use crate::kanjidic::KunPair;

    fn sample() -> KanjidicChar {
        KanjidicChar {
            literal: '一',
            grade: Some(1),
            stroke_count: Some(1),
            korean_h: vec!["일".to_string()],
            on: vec!["イチ".to_string(), "イツ".to_string()],
            kun: vec![KunPair {
                reading: "ひと-つ".to_string(),
                form: "一つ".to_string(),
            }],
            meanings_en: vec!["one".to_string()],
        }
    }

    #[test]
    fn placeholders_mark_unknown_fields() {
        let entry = build_skeleton_entry(&sample());
        assert_eq!(entry.jlpt.as_deref(), Some(TODO));
        assert_eq!(entry.korean.meaning, TODO);
        assert_eq!(entry.last_updated, TODO);
        assert_eq!(entry.korean.reading, "일");
        assert_eq!(entry.meanings, vec!["(en) one".to_string()]);
        assert_eq!(entry.confidence, Confidence::Interpretation);
        assert_eq!(entry.sources[0].name, "KANJIDIC2");
    }

    #[test]
    fn rendered_markdown_round_trips_through_frontmatter_parser() {
        let entry = build_skeleton_entry(&sample());
        let markdown = render_skeleton_markdown(&entry).unwrap();

        assert!(markdown.starts_with("---\n"));
        assert!(markdown.contains("<!-- TODO: 어원 서술 작성 -->"));

        let (parsed, body) = parse_kanji_file(&markdown).expect("스켈레톤은 YAML로서는 유효해야 합니다");
        assert_eq!(parsed.character, "一");
        assert!(body.contains("TODO: 어원 서술 작성"));
    }

    #[test]
    fn missing_meaning_gets_todo_placeholder() {
        let mut k = sample();
        k.meanings_en.clear();
        let entry = build_skeleton_entry(&k);
        assert_eq!(entry.meanings, vec![format!("(en) {TODO}")]);
    }
}

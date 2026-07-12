//! KANJIDIC2 XML에서 글자별 기계적 필드(읽기·획수·학년·의미)를 추출한다.
//!
//! KANJIDIC2는 전자사전연구개발그룹(EDRDG)이 CC-BY-SA 4.0 라이선스로 배포하는
//! 공개 한자 데이터베이스다: <http://www.edrdg.org/wiki/index.php/KANJIDIC_Project>.
//! 이 모듈은 (라이선스 조건에 따라) 네트워크에서 파일을 내려받지 않고, 사용자가
//! 로컬에 준비한 XML 파일 경로만 받아 파싱한다.
//!
//! `jlpt` 필드(구 4급 체계)는 의도적으로 읽지 않는다 — 새 JLPT 5급 체계와
//! 매핑이 불명확해 잘못된 정보를 스켈레톤에 새길 위험이 있기 때문이다
//! (호출부에서 `jlpt: "TODO"` placeholder를 대신 채운다).

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

/// `ja_kun` 훈독 하나. KANJIDIC2 원문은 어간과 오쿠리가나를 `.`으로 구분한다
/// (예: `"まな.ぶ"`). 이 프로젝트 콘텐츠 관례는 `-`를 쓰므로(예: `"まな-ぶ"`,
/// `kanji-schema::KunReading` 문서 참조) 여기서 변환해 둔다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KunPair {
    /// `-`로 어간/오쿠리가나를 구분한 읽기 (오쿠리가나가 없으면 그대로).
    pub reading: String,
    /// 오쿠리가나가 붙은 실제 표기 (예: `"学ぶ"`). 오쿠리가나가 없으면 표제자 그대로.
    pub form: String,
}

/// KANJIDIC2 `<character>` 하나에서 뽑아낸 기계적 필드.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KanjidicChar {
    pub literal: char,
    pub grade: Option<u8>,
    /// 첫 번째 `<stroke_count>`만 채택한다 (후속 항목은 오획수 표기용).
    pub stroke_count: Option<u32>,
    /// 한국 한자음(`korean_h`), 있는 순서 그대로.
    pub korean_h: Vec<String>,
    /// 음독(`ja_on`), 가타카나 그대로.
    pub on: Vec<String>,
    /// 훈독(`ja_kun`), `-` 구분 변환 완료.
    pub kun: Vec<KunPair>,
    /// 영어 의미(`meaning`, `m_lang` 속성이 없거나 `"en"`인 것만).
    pub meanings_en: Vec<String>,
}

/// 다음 `Text` 이벤트를 어느 필드에 채울지 나타낸다.
enum Pending {
    None,
    Literal,
    Grade,
    StrokeCount,
    Reading(String),
    /// `m_lang` 속성 값. 영어가 아니면 텍스트를 버린다.
    Meaning(Option<String>),
}

/// `wanted`에 포함된 글자만 골라 `HashMap<char, KanjidicChar>`로 반환한다.
///
/// XML 전체를 이벤트 스트림으로 훑되(quick-xml, DOM 전체를 메모리에 올리지
/// 않음), `wanted`에 없는 `<character>`는 파싱만 하고 버린다 — 실제
/// KANJIDIC2 전체 파일(13,000자 이상)에서도 특정 글자만 뽑아 쓸 수 있도록.
pub fn parse_wanted(xml: &str, wanted: &HashSet<char>) -> Result<HashMap<char, KanjidicChar>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut out: HashMap<char, KanjidicChar> = HashMap::new();
    let mut buf: Vec<u8> = Vec::new();

    let mut current: Option<KanjidicChar> = None;
    let mut pending = Pending::None;

    loop {
        match reader
            .read_event_into(&mut buf)
            .context("KANJIDIC2 XML 파싱 중 오류가 발생했습니다")?
        {
            Event::Eof => break,

            Event::Start(e) | Event::Empty(e) => {
                let name = e.name();
                let local = name.as_ref();
                match local {
                    b"character" => {
                        current = Some(KanjidicChar::default());
                        pending = Pending::None;
                    }
                    b"literal" if current.is_some() => pending = Pending::Literal,
                    b"grade" if current.is_some() => pending = Pending::Grade,
                    b"stroke_count" if current.is_some() => pending = Pending::StrokeCount,
                    b"reading" if current.is_some() => {
                        let r_type = e
                            .try_get_attribute("r_type")
                            .ok()
                            .flatten()
                            .map(|a| a.unescape_value().unwrap_or_default().into_owned())
                            .unwrap_or_default();
                        pending = Pending::Reading(r_type);
                    }
                    b"meaning" if current.is_some() => {
                        let m_lang = e
                            .try_get_attribute("m_lang")
                            .ok()
                            .flatten()
                            .map(|a| a.unescape_value().unwrap_or_default().into_owned());
                        pending = Pending::Meaning(m_lang);
                    }
                    _ => pending = Pending::None,
                }
            }

            Event::Text(e) => {
                if matches!(pending, Pending::None) {
                    continue;
                }
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }
                if let Some(cur) = current.as_mut() {
                    apply_text(cur, &pending, &text);
                }
                pending = Pending::None;
            }

            Event::End(e) => {
                let local = e.name();
                match local.as_ref() {
                    b"character" => {
                        if let Some(cur) = current.take() {
                            if cur.literal != char::default() && wanted.contains(&cur.literal) {
                                out.insert(cur.literal, cur);
                            }
                        }
                        pending = Pending::None;
                    }
                    b"literal" | b"grade" | b"stroke_count" | b"reading" | b"meaning" => {
                        pending = Pending::None;
                    }
                    _ => {}
                }
            }

            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

fn apply_text(cur: &mut KanjidicChar, pending: &Pending, text: &str) {
    match pending {
        Pending::None => {}
        Pending::Literal => {
            if let Some(c) = text.chars().next() {
                cur.literal = c;
            }
        }
        Pending::Grade => {
            if let Ok(v) = text.parse::<u8>() {
                cur.grade = Some(v);
            }
        }
        Pending::StrokeCount => {
            // KANJIDIC2 관례: 첫 stroke_count만 정확한 획수, 이후는 오획수 표기.
            if cur.stroke_count.is_none() {
                if let Ok(v) = text.parse::<u32>() {
                    cur.stroke_count = Some(v);
                }
            }
        }
        Pending::Reading(r_type) => match r_type.as_str() {
            "ja_on" => cur.on.push(text.to_string()),
            "ja_kun" => cur.kun.push(split_kun(cur.literal, text)),
            "korean_h" => cur.korean_h.push(text.to_string()),
            _ => {}
        },
        Pending::Meaning(m_lang) => {
            // m_lang 속성이 없으면 영어(KANJIDIC2 관례).
            let is_en = m_lang.as_deref().is_none_or_empty_or_en();
            if is_en {
                cur.meanings_en.push(text.to_string());
            }
        }
    }
}

/// `ja_kun` 원문(예: `"まな.ぶ"`, `"いし"`)을 `-` 구분 표기와 오쿠리가나가
/// 붙은 실제 표기로 변환한다. `.`이 없으면 훈독 전체가 표제자 하나로
/// 읽힌다는 뜻이므로 `form`은 표제자 그대로다.
fn split_kun(literal: char, raw: &str) -> KunPair {
    match raw.split_once('.') {
        Some((stem, okurigana)) => KunPair {
            reading: format!("{stem}-{okurigana}"),
            form: format!("{literal}{okurigana}"),
        },
        None => KunPair {
            reading: raw.to_string(),
            form: literal.to_string(),
        },
    }
}

/// `Option<&str>`에 대해 "없거나 영어(en)인가"를 판정하는 작은 확장 트레이트.
/// (KANJIDIC2는 `m_lang` 속성이 없으면 영어를 의미한다.)
trait EnglishOrAbsent {
    fn is_none_or_empty_or_en(&self) -> bool;
}

impl EnglishOrAbsent for Option<&str> {
    fn is_none_or_empty_or_en(&self) -> bool {
        matches!(self, None | Some("") | Some("en"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/kanjidic2/sample.xml");

    fn wanted(chars: &str) -> HashSet<char> {
        chars.chars().collect()
    }

    #[test]
    fn extracts_ichi_fields() {
        let parsed = parse_wanted(FIXTURE, &wanted("一")).unwrap();
        let ichi = parsed.get(&'一').expect("一가 파싱되어야 합니다");

        assert_eq!(ichi.grade, Some(1));
        assert_eq!(ichi.stroke_count, Some(1));
        assert_eq!(ichi.korean_h, vec!["일".to_string()]);
        assert_eq!(ichi.on, vec!["イチ".to_string(), "イツ".to_string()]);
        assert_eq!(
            ichi.kun,
            vec![KunPair {
                reading: "ひと-つ".to_string(),
                form: "一つ".to_string(),
            }]
        );
        assert_eq!(ichi.meanings_en, vec!["one".to_string()]);
    }

    #[test]
    fn takes_first_stroke_count_only() {
        // 二는 픽스처에 stroke_count가 2, 3(오획수) 순서로 두 번 나온다.
        let parsed = parse_wanted(FIXTURE, &wanted("二")).unwrap();
        let ni = parsed.get(&'二').unwrap();
        assert_eq!(ni.stroke_count, Some(2));
    }

    #[test]
    fn ignores_non_english_meanings() {
        // 三에는 m_lang="fr" 의미가 섞여 있으므로 걸러져야 한다.
        let parsed = parse_wanted(FIXTURE, &wanted("三")).unwrap();
        let san = parsed.get(&'三').unwrap();
        assert_eq!(san.meanings_en, vec!["three".to_string()]);
    }

    #[test]
    fn only_returns_wanted_characters() {
        let parsed = parse_wanted(FIXTURE, &wanted("一")).unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(!parsed.contains_key(&'二'));
    }

    #[test]
    fn missing_character_is_simply_absent() {
        let parsed = parse_wanted(FIXTURE, &wanted("火")).unwrap();
        assert!(parsed.is_empty());
    }
}

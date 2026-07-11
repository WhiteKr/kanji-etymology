//! 한자/부수 콘텐츠 frontmatter의 공유 데이터 모델.
//!
//! `serde`(derive)만 의존하며, `crates/build-data`(빌드 파이프라인)와
//! `crates/web`(wasm32 Dioxus 앱)에서 모두 재사용한다.
//! 설계 문서 7장(데이터 모델) 참조: `docs/2026-05-27-kanji-etymology-mvp-design.md`.

use serde::{Deserialize, Serialize};

/// `content/kanji/{character}.md`의 YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KanjiEntry {
    /// 표제 한자 (신자체 등 대표 자형). 파일명과 일치해야 한다.
    pub character: String,

    /// 이체자 목록 (구자체 등).
    #[serde(default)]
    pub variants: Vec<String>,

    /// JLPT 급수 (예: "N5").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jlpt: Option<String>,

    /// 교육 학년 (일본 학년 배당 한자표 기준).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<u8>,

    /// 총 획수.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_count: Option<u32>,

    /// 한국 한자음·훈.
    pub korean: Korean,

    /// 일본어 음독·훈독.
    pub readings: Readings,

    /// 한국어 뜻 목록.
    pub meanings: Vec<String>,

    /// 부품 분해 (구성 요소).
    #[serde(default)]
    pub components: Vec<Component>,

    /// 자형 변천.
    #[serde(default)]
    pub evolution: Vec<Evolution>,

    /// 관련 한자.
    #[serde(default)]
    pub related: Vec<Related>,

    /// 이 한자가 포함된 단어 예시.
    #[serde(default)]
    pub words: Vec<Word>,

    /// 어원 서술의 신뢰도.
    pub confidence: Confidence,

    /// 출처 목록.
    pub sources: Vec<Source>,

    /// 마지막 수정일 (YYYY-MM-DD).
    pub last_updated: String,
}

/// 한국 한자음·훈.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Korean {
    /// 한국 한자음 (예: "학").
    pub reading: String,
    /// 훈+음 형식의 새김 (예: "배울 학").
    pub meaning: String,
}

/// 일본어 음독·훈독.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Readings {
    /// 음독 (가타카나).
    #[serde(default)]
    pub on: Vec<String>,
    /// 훈독.
    #[serde(default)]
    pub kun: Vec<KunReading>,
}

/// 훈독 하나. `reading`은 어간-오쿠리가나를 `-`로 구분해 표기할 수 있다
/// (예: `"まな-ぶ"`), `form`은 오쿠리가나가 붙은 실제 표기(예: `"学ぶ"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KunReading {
    pub reading: String,
    pub form: String,
}

/// 부품 분해 요소.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    /// 부품 자형.
    pub form: String,
    /// 이 부품의 역할·의미 설명.
    pub role: String,
    /// 연결 링크. `/kanji/{한자}` 또는 `/radical/{부수}` 형식, 없으면 `null`.
    #[serde(default)]
    pub link: Option<String>,
}

/// 자형 변천 단계 하나.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evolution {
    /// 시대 (예: "갑골문").
    pub era: String,
    /// 그 시대의 자형.
    pub form: String,
    /// 글리프 출처 식별자.
    pub source: String,
}

/// 관련 한자 하나.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Related {
    /// 관련 한자.
    pub kanji: String,
    /// 관계 설명 (예: "동일 부품(子)").
    pub relation: String,
}

/// 이 한자가 포함된 단어 예시 하나.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Word {
    /// 단어 표기.
    pub word: String,
    /// 단어 읽기 (가나).
    pub reading: String,
    /// 한국어 뜻풀이.
    pub gloss: String,
}

/// 어원 서술의 신뢰도. 학설이 확립되어 있는지, 해석인지, 추정인지를 구분한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// 정설로 널리 받아들여짐.
    WellAttested,
    /// 여러 학설 중 하나의 해석.
    Interpretation,
    /// 근거가 약한 추정.
    Tentative,
}

/// 출처 하나.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// 출처명 (예: "說文解字", "KANJIDIC2").
    pub name: String,
    /// 권·장 등 세부 참조. `ref`는 Rust 예약어이므로 `r#ref`로 받고
    /// YAML/JSON 상의 키는 `ref`로 유지한다.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    /// 라이선스 (예: "CC-BY-SA").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// 참조 URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 발행 연도.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

/// `content/radicals/{radical}.md`의 YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadicalEntry {
    /// 표제 부수 문자. 파일명과 일치해야 한다.
    pub radical: String,
    /// 한국에서 통용되는 부수명 (예: "삼수변").
    pub name: String,
    /// 부수가 나타내는 의미.
    pub meaning: String,
    /// 획수.
    pub stroke_count: u32,
    /// 이체자 형태 (선택).
    #[serde(default)]
    pub variants: Vec<String>,
    /// 마지막 수정일 (YYYY-MM-DD).
    pub last_updated: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_serializes_as_kebab_case() {
        assert_eq!(
            serde_json::to_string(&Confidence::WellAttested).unwrap(),
            "\"well-attested\""
        );
    }
}

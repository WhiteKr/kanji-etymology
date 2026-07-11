//! YAML frontmatter + Markdown 본문 분리, 그리고 콘텐츠 타입으로의 파싱.

use anyhow::{anyhow, Context, Result};
use kanji_schema::{KanjiEntry, RadicalEntry};

/// `---\n<YAML>\n---\n<본문>` 형식의 원문을 `(YAML 원문, 본문)`으로 분리한다.
///
/// - 여는 구분자(`---`)는 파일 맨 앞줄이어야 한다.
/// - 닫는 구분자는 자신의 줄에 단독으로 `---`만 있어야 한다 (다음 문자가
///   개행이거나 파일 끝).
/// - CRLF는 LF로 정규화한 뒤 처리한다.
pub fn split_frontmatter(raw: &str) -> Result<(String, String)> {
    let normalized = raw.replace("\r\n", "\n");

    let after_open = normalized
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("frontmatter 시작 구분자('---')가 파일 첫 줄에 없습니다"))?;

    let mut search_from = 0usize;
    loop {
        let rel = after_open[search_from..].find("\n---").ok_or_else(|| {
            anyhow!("frontmatter 종료 구분자('---')를 찾을 수 없습니다")
        })?;
        let abs = search_from + rel;
        let after_marker = abs + "\n---".len();

        // 종료 구분자 줄에는 '---' 외의 다른 내용이 없어야 한다.
        let boundary_ok =
            after_marker == after_open.len() || after_open[after_marker..].starts_with('\n');

        if boundary_ok {
            let yaml = after_open[..abs].to_string();
            let body_start = if after_open[after_marker..].starts_with('\n') {
                after_marker + 1
            } else {
                after_marker
            };
            let body = after_open[body_start..].trim_start_matches('\n').to_string();
            return Ok((yaml, body));
        }

        search_from = after_marker;
    }
}

/// 한자 콘텐츠 파일(`content/kanji/{character}.md`)을 파싱한다.
pub fn parse_kanji_file(raw: &str) -> Result<(KanjiEntry, String)> {
    let (yaml, body) = split_frontmatter(raw)?;
    let entry: KanjiEntry =
        serde_yaml::from_str(&yaml).context("frontmatter YAML을 KanjiEntry로 파싱하는 데 실패했습니다")?;
    Ok((entry, body))
}

/// 부수 콘텐츠 파일(`content/radicals/{radical}.md`)을 파싱한다.
pub fn parse_radical_file(raw: &str) -> Result<(RadicalEntry, String)> {
    let (yaml, body) = split_frontmatter(raw)?;
    let entry: RadicalEntry = serde_yaml::from_str(&yaml)
        .context("frontmatter YAML을 RadicalEntry로 파싱하는 데 실패했습니다")?;
    Ok((entry, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_frontmatter() {
        let raw = "---\ncharacter: \"学\"\n---\n# 제목\n본문 내용\n";
        let (yaml, body) = split_frontmatter(raw).unwrap();
        assert_eq!(yaml, "character: \"学\"");
        assert_eq!(body, "# 제목\n본문 내용\n");
    }

    #[test]
    fn handles_crlf_line_endings() {
        let raw = "---\r\ncharacter: \"学\"\r\n---\r\n본문\r\n";
        let (yaml, body) = split_frontmatter(raw).unwrap();
        assert_eq!(yaml, "character: \"学\"");
        assert_eq!(body, "본문\n");
    }

    #[test]
    fn errors_without_opening_delimiter() {
        let raw = "character: \"学\"\n---\n본문\n";
        assert!(split_frontmatter(raw).is_err());
    }

    #[test]
    fn errors_without_closing_delimiter() {
        let raw = "---\ncharacter: \"学\"\n본문만 있음\n";
        assert!(split_frontmatter(raw).is_err());
    }

    #[test]
    fn tolerates_dashes_inside_yaml_values() {
        // YAML 값 내부의 "---"는 줄 경계에서 시작하지 않으므로 종료
        // 구분자로 오인되지 않아야 한다.
        let raw = "---\nnote: \"a---b\"\ncharacter: \"学\"\n---\n본문\n";
        let (yaml, _) = split_frontmatter(raw).unwrap();
        assert!(yaml.contains("a---b"));
        assert!(yaml.contains("character"));
    }
}

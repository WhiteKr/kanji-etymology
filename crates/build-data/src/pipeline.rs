//! 파이프라인 오케스트레이션: 파싱 → 검증 → 인덱스 생성 → 출력.
//!
//! 검증에 실패한 파일이 있으면 어떤 인덱스도 생성하지 않고, 모아둔 오류를
//! 전부 담은 `Err`를 반환한다 (부분적으로 잘못된 데이터가 배포되는 일이 없도록).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use kanji_schema::{KanjiEntry, RadicalEntry};
use serde::Serialize;

use crate::frontmatter::{parse_kanji_file, parse_radical_file};
use crate::index::{
    build_by_radical_index, build_by_word_index, build_kanji_list, build_radicals_list,
    build_search_index,
};

/// 파이프라인 실행 결과 요약.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub kanji_count: usize,
    pub radical_count: usize,
}

struct ParsedKanji {
    entry: KanjiEntry,
    body: String,
}

struct ParsedRadical {
    entry: RadicalEntry,
    body: String,
}

#[derive(Serialize)]
struct KanjiOutput<'a> {
    #[serde(flatten)]
    entry: &'a KanjiEntry,
    body_markdown: &'a str,
}

#[derive(Serialize)]
struct RadicalOutput<'a> {
    #[serde(flatten)]
    entry: &'a RadicalEntry,
    body_markdown: &'a str,
}

/// `content_dir` 아래 `kanji/`, `radicals/` 콘텐츠를 파싱·검증하고
/// `out_dir`에 검색 인덱스·역인덱스·개별 JSON을 생성한다.
pub fn run(content_dir: &Path, out_dir: &Path) -> Result<Summary> {
    let kanji_dir = content_dir.join("kanji");
    let radicals_dir = content_dir.join("radicals");

    let mut errors: Vec<String> = Vec::new();

    // --- 1. 파싱 + 기본 검증 (파일명 일치, 중복 character/radical) ---
    let mut kanji_entries: Vec<ParsedKanji> = Vec::new();
    let mut seen_kanji: HashSet<String> = HashSet::new();

    for path in list_md_files(&kanji_dir)? {
        let file_name = file_stem_string(&path);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("{}: 파일을 읽을 수 없습니다", path.display()))?;

        match parse_kanji_file(&raw) {
            Ok((entry, body)) => {
                if entry.character != file_name {
                    errors.push(format!(
                        "{}: frontmatter의 character(\"{}\")가 파일명(\"{}\")과 일치하지 않습니다",
                        path.display(),
                        entry.character,
                        file_name
                    ));
                }
                if !seen_kanji.insert(entry.character.clone()) {
                    errors.push(format!(
                        "{}: 한자 \"{}\"가 다른 파일과 중복됩니다",
                        path.display(),
                        entry.character
                    ));
                }
                kanji_entries.push(ParsedKanji { entry, body });
            }
            Err(e) => errors.push(format!("{}: {:#}", path.display(), e)),
        }
    }

    // --- 2. 부수 파싱 + 기본 검증 ---
    let mut radical_entries: Vec<ParsedRadical> = Vec::new();
    let mut seen_radicals: HashSet<String> = HashSet::new();

    for path in list_md_files(&radicals_dir)? {
        let file_name = file_stem_string(&path);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("{}: 파일을 읽을 수 없습니다", path.display()))?;

        match parse_radical_file(&raw) {
            Ok((entry, body)) => {
                if entry.radical != file_name {
                    errors.push(format!(
                        "{}: frontmatter의 radical(\"{}\")이 파일명(\"{}\")과 일치하지 않습니다",
                        path.display(),
                        entry.radical,
                        file_name
                    ));
                }
                if !seen_radicals.insert(entry.radical.clone()) {
                    errors.push(format!(
                        "{}: 부수 \"{}\"가 다른 파일과 중복됩니다",
                        path.display(),
                        entry.radical
                    ));
                }
                radical_entries.push(ParsedRadical { entry, body });
            }
            Err(e) => errors.push(format!("{}: {:#}", path.display(), e)),
        }
    }

    // --- 3. 참조 무결성 검증 (모든 파일을 다 읽은 뒤에만 판단 가능) ---
    for pk in &kanji_entries {
        for component in &pk.entry.components {
            let Some(link) = &component.link else { continue };
            if let Some(target) = link.strip_prefix("/kanji/") {
                if !seen_kanji.contains(target) {
                    errors.push(format!(
                        "content/kanji/{}.md: components 링크가 존재하지 않는 한자를 가리킵니다: {}",
                        pk.entry.character, link
                    ));
                }
            } else if let Some(target) = link.strip_prefix("/radical/") {
                if !seen_radicals.contains(target) {
                    errors.push(format!(
                        "content/kanji/{}.md: components 링크가 존재하지 않는 부수를 가리킵니다: {}",
                        pk.entry.character, link
                    ));
                }
            } else {
                errors.push(format!(
                    "content/kanji/{}.md: 지원하지 않는 링크 형식입니다 (/kanji/... 또는 /radical/...만 허용): {}",
                    pk.entry.character, link
                ));
            }
        }

        for related in &pk.entry.related {
            if !seen_kanji.contains(&related.kanji) {
                errors.push(format!(
                    "content/kanji/{}.md: related.kanji가 존재하지 않는 한자를 가리킵니다: {}",
                    pk.entry.character, related.kanji
                ));
            }
        }
    }

    if !errors.is_empty() {
        bail!(errors.join("\n"));
    }

    // --- 4. 출력 생성 (검증을 모두 통과했을 때만 실행) ---
    fs::create_dir_all(out_dir)
        .with_context(|| format!("{}: 출력 디렉터리를 만들 수 없습니다", out_dir.display()))?;
    let kanji_out_dir = out_dir.join("kanji");
    let radicals_out_dir = out_dir.join("radicals");
    fs::create_dir_all(&kanji_out_dir)?;
    fs::create_dir_all(&radicals_out_dir)?;

    let kanji_only: Vec<KanjiEntry> = kanji_entries.iter().map(|p| p.entry.clone()).collect();
    let radicals_only: Vec<RadicalEntry> = radical_entries.iter().map(|p| p.entry.clone()).collect();
    let by_radical = build_by_radical_index(&kanji_only);

    write_json(&out_dir.join("search-index.json"), &build_search_index(&kanji_only))?;
    write_json(&out_dir.join("kanji-list.json"), &build_kanji_list(&kanji_only))?;
    // 부수 인덱스 페이지(/radicals)용 — 부수별 소속 한자 수를 함께 담는다.
    write_json(
        &out_dir.join("radicals-list.json"),
        &build_radicals_list(&radicals_only, &by_radical),
    )?;
    write_json(&out_dir.join("by-radical.json"), &by_radical)?;
    write_json(&out_dir.join("by-word.json"), &build_by_word_index(&kanji_only))?;

    for pk in &kanji_entries {
        let output = KanjiOutput {
            entry: &pk.entry,
            body_markdown: &pk.body,
        };
        write_json(&kanji_out_dir.join(format!("{}.json", pk.entry.character)), &output)?;
    }

    for pr in &radical_entries {
        let output = RadicalOutput {
            entry: &pr.entry,
            body_markdown: &pr.body,
        };
        write_json(
            &radicals_out_dir.join(format!("{}.json", pr.entry.radical)),
            &output,
        )?;
    }

    Ok(Summary {
        kanji_count: kanji_entries.len(),
        radical_count: radical_entries.len(),
    })
}

/// 디렉터리 안의 `.md` 파일 목록을 결정적인(정렬된) 순서로 반환한다.
/// 디렉터리가 아예 없으면 빈 목록으로 취급한다 (예: radicals 없이 kanji만 있는 경우).
fn list_md_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("{}: 디렉터리를 읽을 수 없습니다", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    files.sort();
    Ok(files)
}

fn file_stem_string(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .with_context(|| format!("{}: JSON 직렬화에 실패했습니다", path.display()))?;
    fs::write(path, json).with_context(|| format!("{}: 파일 쓰기에 실패했습니다", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn temp_out_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "build-data-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn valid_fixture_succeeds_and_produces_expected_files() {
        let content_dir = fixtures_dir().join("valid");
        let out_dir = temp_out_dir("valid");

        let summary = run(&content_dir, &out_dir).expect("정상 fixture는 성공해야 합니다");
        assert!(summary.kanji_count >= 2);
        assert!(summary.radical_count >= 1);

        assert!(out_dir.join("search-index.json").exists());
        assert!(out_dir.join("kanji-list.json").exists());
        assert!(out_dir.join("by-radical.json").exists());
        assert!(out_dir.join("by-word.json").exists());
        assert!(out_dir.join("kanji/学.json").exists());

        let search_index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(out_dir.join("search-index.json")).unwrap())
                .unwrap();
        assert_eq!(search_index["by_kanji"]["学"], "学");
        assert_eq!(search_index["by_kr_sound"]["학"][0], "学");
        // まなぶ (훈독 정규화) 로 검색 가능해야 한다.
        assert!(search_index["by_kun"]["まなぶ"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "学"));
        // gaku (음독 ガク의 로마자) 로 검색 가능해야 한다.
        assert!(search_index["by_romaji"]["gaku"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "学"));

        // 부수 인덱스 페이지용 radicals-list.json — 부수 요약 + 소속 한자 수.
        let radicals_list: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(out_dir.join("radicals-list.json")).unwrap())
                .unwrap();
        let radicals_arr = radicals_list.as_array().unwrap();
        assert_eq!(radicals_arr.len(), summary.radical_count);
        let mie = radicals_arr
            .iter()
            .find(|r| r["radical"] == "冖")
            .expect("fixture의 冖 부수가 radicals-list에 있어야 합니다");
        assert_eq!(mie["name"], "민갓머리");
        // 学이 冖을 부품으로 가지므로 소속 한자 수는 1 이상이어야 한다.
        assert!(mie["kanji_count"].as_u64().unwrap() >= 1);

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn missing_required_field_fails() {
        let content_dir = fixtures_dir().join("invalid-missing-field");
        let out_dir = temp_out_dir("missing-field");

        let result = run(&content_dir, &out_dir);
        assert!(result.is_err(), "필수 필드 누락은 실패해야 합니다");
        assert!(!out_dir.exists(), "실패 시 출력 디렉터리를 만들면 안 됩니다");
    }

    #[test]
    fn broken_link_fails() {
        let content_dir = fixtures_dir().join("invalid-broken-link");
        let out_dir = temp_out_dir("broken-link");

        let result = run(&content_dir, &out_dir);
        let err = result.expect_err("존재하지 않는 링크 대상은 실패해야 합니다");
        assert!(err.to_string().contains("존재하지 않는"));
        assert!(!out_dir.exists());
    }

    #[test]
    fn filename_character_mismatch_fails() {
        let content_dir = fixtures_dir().join("invalid-filename-mismatch");
        let out_dir = temp_out_dir("filename-mismatch");

        let result = run(&content_dir, &out_dir);
        let err = result.expect_err("파일명과 character 불일치는 실패해야 합니다");
        assert!(err.to_string().contains("일치하지"));
    }

    #[test]
    fn duplicate_character_fails() {
        let content_dir = fixtures_dir().join("invalid-duplicate");
        let out_dir = temp_out_dir("duplicate");

        let result = run(&content_dir, &out_dir);
        let err = result.expect_err("character 중복은 실패해야 합니다");
        assert!(err.to_string().contains("중복"));
    }
}

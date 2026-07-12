//! `build-data import-kanjidic` 서브커맨드의 오케스트레이션.
//!
//! KANJIDIC2 XML(로컬 파일)에서 요청된 글자들의 기계적 필드를 뽑아
//! `content/kanji/{글자}.md`와 같은 형식의 초안 스켈레톤을 `out_dir`에
//! 만든다. 어원 서술은 채우지 않는다 — `skeleton` 모듈 문서 참조.
//!
//! KANJIDIC2는 EDRDG가 CC-BY-SA 4.0로 배포한다. 이 도구는 라이선스 조건을
//! 지키기 위해 **네트워크에서 파일을 내려받지 않는다**: `--xml`로 사용자가
//! 미리 받아 둔 로컬 파일 경로를 넘겨야 한다.
//! (배포처: <http://www.edrdg.org/wiki/index.php/KANJIDIC_Project>)

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::kanjidic;
use crate::skeleton::{build_skeleton_entry, render_skeleton_markdown};

/// `import-kanjidic` 실행에 필요한 입력.
pub struct ImportOptions {
    /// KANJIDIC2 XML 파일 경로 (로컬).
    pub xml_path: PathBuf,
    /// 뽑아낼 글자 목록. 중복은 첫 등장 순서를 유지한 채 제거된다.
    pub chars: Vec<char>,
    /// 스켈레톤 `.md` 파일을 쓸 디렉터리.
    pub out_dir: PathBuf,
    /// 이미 `content_dir/kanji/{글자}.md`가 있으면 건너뛴다.
    pub skip_existing: bool,
    /// `skip_existing`을 판단할 때 기준이 되는 콘텐츠 디렉터리
    /// (`content_dir/kanji/{글자}.md` 존재 여부를 확인).
    pub content_dir: PathBuf,
}

/// 실행 결과 요약.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    /// 스켈레톤을 새로 쓴 글자 (요청 순서).
    pub written: Vec<char>,
    /// `--skip-existing`으로 건너뛴 글자 (요청 순서).
    pub skipped: Vec<char>,
}

/// KANJIDIC2 XML을 읽고, 요청된 글자들의 스켈레톤을 `out_dir`에 생성한다.
///
/// 요청된 글자 중 하나라도 XML에서 찾지 못하면 어떤 파일도 쓰지 않고
/// 에러를 반환한다 (파이프라인의 "부분 실패 없음" 원칙과 동일).
pub fn run(opts: &ImportOptions) -> Result<ImportSummary> {
    let xml = fs::read_to_string(&opts.xml_path).with_context(|| {
        format!(
            "{}: KANJIDIC2 XML 파일을 읽을 수 없습니다",
            opts.xml_path.display()
        )
    })?;

    // 요청 순서를 보존한 채 중복 글자를 제거한다.
    let mut ordered_unique: Vec<char> = Vec::new();
    let mut seen: HashSet<char> = HashSet::new();
    for &c in &opts.chars {
        if seen.insert(c) {
            ordered_unique.push(c);
        }
    }

    if ordered_unique.is_empty() {
        bail!("--chars로 최소 한 글자 이상 지정해야 합니다");
    }

    let wanted: HashSet<char> = ordered_unique.iter().copied().collect();
    let parsed = kanjidic::parse_wanted(&xml, &wanted).with_context(|| {
        format!(
            "{}: KANJIDIC2 XML을 파싱하는 데 실패했습니다",
            opts.xml_path.display()
        )
    })?;

    let missing: Vec<char> = ordered_unique
        .iter()
        .copied()
        .filter(|c| !parsed.contains_key(c))
        .collect();
    if !missing.is_empty() {
        let missing_str: String = missing.iter().collect();
        bail!(
            "{}: KANJIDIC2에서 찾을 수 없는 글자입니다: {missing_str}",
            opts.xml_path.display()
        );
    }

    fs::create_dir_all(&opts.out_dir).with_context(|| {
        format!(
            "{}: 출력 디렉터리를 만들 수 없습니다",
            opts.out_dir.display()
        )
    })?;

    let mut summary = ImportSummary::default();

    for c in ordered_unique {
        if opts.skip_existing && existing_content_path(&opts.content_dir, c).exists() {
            summary.skipped.push(c);
            continue;
        }

        let kd = parsed
            .get(&c)
            .expect("missing 검사를 이미 통과했으므로 항상 존재해야 합니다");
        let entry = build_skeleton_entry(kd);
        let markdown = render_skeleton_markdown(&entry)?;

        let out_path = opts.out_dir.join(format!("{c}.md"));
        fs::write(&out_path, markdown)
            .with_context(|| format!("{}: 파일 쓰기에 실패했습니다", out_path.display()))?;
        summary.written.push(c);
    }

    Ok(summary)
}

fn existing_content_path(content_dir: &Path, c: char) -> PathBuf {
    content_dir.join("kanji").join(format!("{c}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_XML: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/kanjidic2/sample.xml"
    );

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "build-data-import-kanjidic-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_skeleton_for_each_requested_char() {
        let out_dir = temp_dir("writes");
        let content_dir = temp_dir("writes-content"); // 존재는 하지만 kanji/ 하위는 비어 있음

        let opts = ImportOptions {
            xml_path: PathBuf::from(FIXTURE_XML),
            chars: vec!['一', '二', '三'],
            out_dir: out_dir.clone(),
            skip_existing: false,
            content_dir,
        };

        let summary = run(&opts).expect("정상 픽스처는 성공해야 합니다");
        assert_eq!(summary.written, vec!['一', '二', '三']);
        assert!(summary.skipped.is_empty());

        for c in ['一', '二', '三'] {
            let path = out_dir.join(format!("{c}.md"));
            assert!(path.exists(), "{c} 스켈레톤 파일이 있어야 합니다");
            let content = fs::read_to_string(&path).unwrap();
            assert!(content.contains("jlpt: TODO") || content.contains("jlpt: \"TODO\""));
            assert!(content.contains("<!-- TODO: 어원 서술 작성 -->"));
        }

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn deduplicates_requested_chars_preserving_order() {
        let out_dir = temp_dir("dedup");
        let content_dir = temp_dir("dedup-content");

        let opts = ImportOptions {
            xml_path: PathBuf::from(FIXTURE_XML),
            chars: vec!['一', '二', '一'],
            out_dir: out_dir.clone(),
            skip_existing: false,
            content_dir,
        };

        let summary = run(&opts).unwrap();
        assert_eq!(summary.written, vec!['一', '二']);

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn skip_existing_skips_chars_already_in_content_dir() {
        let out_dir = temp_dir("skip-out");
        let content_dir = temp_dir("skip-content");
        fs::create_dir_all(content_dir.join("kanji")).unwrap();
        fs::write(content_dir.join("kanji").join("一.md"), "이미 존재하는 콘텐츠").unwrap();

        let opts = ImportOptions {
            xml_path: PathBuf::from(FIXTURE_XML),
            chars: vec!['一', '二'],
            out_dir: out_dir.clone(),
            skip_existing: true,
            content_dir,
        };

        let summary = run(&opts).unwrap();
        assert_eq!(summary.written, vec!['二']);
        assert_eq!(summary.skipped, vec!['一']);
        assert!(!out_dir.join("一.md").exists());
        assert!(out_dir.join("二.md").exists());

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn missing_character_fails_with_no_files_written() {
        let out_dir = temp_dir("missing");
        let content_dir = temp_dir("missing-content");

        let opts = ImportOptions {
            xml_path: PathBuf::from(FIXTURE_XML),
            chars: vec!['一', '火'], // 火는 픽스처 XML에 없음
            out_dir: out_dir.clone(),
            skip_existing: false,
            content_dir,
        };

        let err = run(&opts).expect_err("존재하지 않는 글자 요청은 실패해야 합니다");
        let message = err.to_string();
        assert!(
            message.contains("찾을 수 없는 글자") && message.contains('火'),
            "에러 메시지에 누락된 글자가 포함되어야 합니다: {message}"
        );
        assert!(
            !out_dir.join("一.md").exists(),
            "일부라도 실패하면 아무 파일도 쓰지 않아야 합니다"
        );

        let _ = fs::remove_dir_all(&out_dir);
    }

    #[test]
    fn missing_xml_file_gives_readable_error() {
        let out_dir = temp_dir("missing-xml");
        let content_dir = temp_dir("missing-xml-content");

        let opts = ImportOptions {
            xml_path: PathBuf::from("does-not-exist.xml"),
            chars: vec!['一'],
            out_dir,
            skip_existing: false,
            content_dir,
        };

        let err = run(&opts).expect_err("존재하지 않는 XML 경로는 실패해야 합니다");
        assert!(err.to_string().contains("읽을 수 없습니다"));
    }
}

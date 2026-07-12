//! 빌드 시 데이터 파이프라인 CLI 진입점.
//!
//! 실제 로직은 `build_data`(lib) 크레이트에 있다. 이 파일은 인자 파싱과
//! 종료 코드 결정만 담당하는 얇은 래퍼다.
//!
//! 사용법 (기본 동작, 하위 호환):
//!   `build-data [--content-dir <경로>] [--out-dir <경로>]`
//!   기본값: `--content-dir content`, `--out-dir dist/data`
//!
//! 사용법 (서브커맨드, KANJIDIC2 임포트):
//!   `build-data import-kanjidic --xml <경로> --chars <글자들> --out-dir <경로> [--skip-existing --content-dir <경로>]`
//!   자세한 옵션: `build-data import-kanjidic --help`

use std::path::PathBuf;

use build_data::import_kanjidic::{self, ImportOptions};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("import-kanjidic") {
        run_import_kanjidic(&args[1..]);
        return;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_top_level_help();
        return;
    }

    let mut content_dir = PathBuf::from("content");
    let mut out_dir = PathBuf::from("dist/data");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--content-dir" => {
                i += 1;
                match args.get(i) {
                    Some(v) => content_dir = PathBuf::from(v),
                    None => fail("--content-dir 뒤에 경로가 필요합니다"),
                }
            }
            "--out-dir" => {
                i += 1;
                match args.get(i) {
                    Some(v) => out_dir = PathBuf::from(v),
                    None => fail("--out-dir 뒤에 경로가 필요합니다"),
                }
            }
            other => fail(&format!("알 수 없는 인자입니다: {other}")),
        }
        i += 1;
    }

    match build_data::run(&content_dir, &out_dir) {
        Ok(summary) => {
            println!(
                "처리 완료: 한자 {}자, 부수 {}개",
                summary.kanji_count, summary.radical_count
            );
        }
        Err(e) => {
            eprintln!("build-data: 빌드 검증 실패\n{e}");
            std::process::exit(1);
        }
    }
}

/// `import-kanjidic` 서브커맨드의 인자 파싱 및 실행.
fn run_import_kanjidic(args: &[String]) {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_import_kanjidic_help();
        return;
    }

    let mut xml_path: Option<PathBuf> = None;
    let mut chars_arg: Option<String> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut skip_existing = false;
    let mut content_dir = PathBuf::from("content");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--xml" => {
                i += 1;
                xml_path = Some(PathBuf::from(arg_value(args, i, "--xml")));
            }
            "--chars" => {
                i += 1;
                chars_arg = Some(arg_value(args, i, "--chars").to_string());
            }
            "--out-dir" => {
                i += 1;
                out_dir = Some(PathBuf::from(arg_value(args, i, "--out-dir")));
            }
            "--content-dir" => {
                i += 1;
                content_dir = PathBuf::from(arg_value(args, i, "--content-dir"));
            }
            "--skip-existing" => {
                skip_existing = true;
            }
            other => fail(&format!("import-kanjidic: 알 수 없는 인자입니다: {other}")),
        }
        i += 1;
    }

    let xml_path =
        xml_path.unwrap_or_else(|| fail("import-kanjidic: --xml 인자가 필요합니다 (KANJIDIC2 XML 경로)"));
    let chars_arg =
        chars_arg.unwrap_or_else(|| fail("import-kanjidic: --chars 인자가 필요합니다 (예: --chars 一二三)"));
    let out_dir = out_dir.unwrap_or_else(|| fail("import-kanjidic: --out-dir 인자가 필요합니다"));

    let chars: Vec<char> = chars_arg.chars().collect();
    if chars.is_empty() {
        fail("import-kanjidic: --chars에 최소 한 글자 이상 지정해야 합니다");
    }

    let opts = ImportOptions {
        xml_path,
        chars,
        out_dir,
        skip_existing,
        content_dir,
    };

    match import_kanjidic::run(&opts) {
        Ok(summary) => {
            println!(
                "import-kanjidic 완료: 스켈레톤 생성 {}자{}",
                summary.written.len(),
                if summary.skipped.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", 이미 존재하여 건너뜀 {}자 ({})",
                        summary.skipped.len(),
                        summary.skipped.iter().collect::<String>()
                    )
                }
            );
            if !summary.written.is_empty() {
                println!(
                    "  생성됨: {}",
                    summary.written.iter().collect::<String>()
                );
            }
        }
        Err(e) => {
            eprintln!("build-data import-kanjidic: 실패\n{e:#}");
            std::process::exit(1);
        }
    }
}

fn arg_value<'a>(args: &'a [String], i: usize, flag: &str) -> &'a str {
    match args.get(i) {
        Some(v) => v.as_str(),
        None => fail(&format!("{flag} 뒤에 값이 필요합니다")),
    }
}

fn print_top_level_help() {
    println!("사용법:");
    println!("  build-data [--content-dir <경로>] [--out-dir <경로>]");
    println!("      콘텐츠(content/kanji, content/radicals)를 검증하고 검색 인덱스·개별 JSON을 생성합니다.");
    println!("      기본값: --content-dir content, --out-dir dist/data");
    println!();
    println!("  build-data import-kanjidic --xml <경로> --chars <글자들> --out-dir <경로> [옵션]");
    println!("      KANJIDIC2 XML에서 초안 스켈레톤 마크다운을 생성합니다.");
    println!("      자세한 옵션: build-data import-kanjidic --help");
}

fn print_import_kanjidic_help() {
    println!(
        "사용법: build-data import-kanjidic --xml <경로> --chars <글자들> --out-dir <경로> [옵션]"
    );
    println!();
    println!("KANJIDIC2 XML에서 글자별 기계적 필드(음독·훈독·한국음·획수·학년·영어 의미)를");
    println!("뽑아 content/kanji/{{글자}}.md 형식의 초안 스켈레톤을 생성합니다.");
    println!("어원 서술(본문)은 <!-- TODO: 어원 서술 작성 --> 마커로 남기며, jlpt/한국훈/");
    println!("last_updated 등 KANJIDIC2에 없는 값은 \"TODO\" placeholder로 채웁니다.");
    println!("생성된 파일은 build-data의 콘텐츠 검증을 통과하지 못합니다(의도된 동작) —");
    println!("TODO를 실제 값으로 채운 뒤 content/kanji/로 옮겨야 정식 콘텐츠가 됩니다.");
    println!();
    println!("옵션:");
    println!("  --xml <경로>          KANJIDIC2 XML 파일 경로 (로컬 파일만 지원; 필수)");
    println!("  --chars <글자들>      스켈레톤을 생성할 글자, 구분자 없이 이어 붙여 지정 (예: 一二三; 필수)");
    println!("  --out-dir <경로>      생성한 .md 스켈레톤을 쓸 디렉터리 (필수)");
    println!("  --skip-existing       <content-dir>/kanji/{{글자}}.md가 이미 있으면 그 글자는 건너뜀");
    println!("  --content-dir <경로>  --skip-existing 판단 기준 콘텐츠 디렉터리 (기본값: content)");
    println!();
    println!("예시:");
    println!(
        "  build-data import-kanjidic --xml kanjidic2.xml --chars 一二三 --out-dir drafts --skip-existing"
    );
    println!();
    println!("라이선스 고지 (KANJIDIC2):");
    println!("  KANJIDIC2는 전자사전연구개발그룹(Electronic Dictionary Research and");
    println!("  Development Group, EDRDG)이 CC-BY-SA 4.0 조건으로 배포하는 공개 한자");
    println!("  데이터베이스입니다. 배포처: http://www.edrdg.org/wiki/index.php/KANJIDIC_Project");
    println!("  이 도구는 라이선스 조건을 지키기 위해 KANJIDIC2 파일을 네트워크에서");
    println!("  내려받지 않습니다 — 사용자가 위 배포처에서 직접 내려받은 로컬 파일 경로를");
    println!("  --xml로 넘겨야 합니다. 생성된 스켈레톤의 sources에는 KANJIDIC2 출처와");
    println!("  라이선스가 자동으로 기록됩니다.");
}

fn fail(message: &str) -> ! {
    eprintln!("build-data: {message}");
    std::process::exit(1);
}

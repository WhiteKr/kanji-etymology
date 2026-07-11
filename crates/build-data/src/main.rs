//! 빌드 시 데이터 파이프라인 CLI 진입점.
//!
//! 실제 로직은 `build_data`(lib) 크레이트에 있다. 이 파일은 인자 파싱과
//! 종료 코드 결정만 담당하는 얇은 래퍼다.
//!
//! 사용법: `build-data [--content-dir <경로>] [--out-dir <경로>]`
//! 기본값: `--content-dir content`, `--out-dir dist/data`

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

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

fn fail(message: &str) -> ! {
    eprintln!("build-data: {message}");
    std::process::exit(1);
}

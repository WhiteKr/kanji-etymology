//! 가나 → 헵번식 로마자 변환.
//!
//! 히라가나·가타카나를 모두 지원한다. 요음(きゃ 등), 촉음(っ→자음 중복),
//! 장음(ー→직전 모음 반복), ん(뒤따르는 소리에 따라 n/m/n' 로 분기) 처리를
//! 포함한다. 검색 인덱스 생성용으로 쓰이므로 마크론(ō, ū) 대신 모음을
//! 겹쳐 쓰는 단순화된 헵번식을 사용한다 (예: コーヒー → "koohii").

/// 가나 문자열을 헵번식 로마자로 변환한다. 가나가 아닌 문자는 그대로 통과시킨다.
pub fn kana_to_romaji(input: &str) -> String {
    let hiragana = katakana_to_hiragana(input);
    let morae = tokenize_morae(&hiragana);
    morae_to_romaji(&morae)
}

/// 가타카나(U+30A1–U+30F6)를 대응하는 히라가나로 변환한다.
/// 장음 부호(ー, U+30FC)는 히라가나에 대응 문자가 없으므로 그대로 둔다.
fn katakana_to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            if (0x30A1..=0x30F6).contains(&cp) {
                char::from_u32(cp - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

fn is_small_y(c: char) -> bool {
    matches!(c, 'ゃ' | 'ゅ' | 'ょ')
}

/// 소문자 や/ゆ/よ와 결합해 요음을 이룰 수 있는 자음 가나.
fn combines_with_small_y(c: char) -> bool {
    matches!(
        c,
        'き' | 'ぎ' | 'し' | 'じ' | 'ち' | 'ぢ' | 'に' | 'ひ' | 'び' | 'ぴ' | 'み' | 'り'
    )
}

/// 히라가나 문자열을 모라(음절) 단위 토큰으로 나눈다. 요음은 두 글자를
/// 하나의 토큰으로 묶는다.
fn tokenize_morae(hira: &str) -> Vec<String> {
    let chars: Vec<char> = hira.chars().collect();
    let mut morae = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if combines_with_small_y(c) && i + 1 < chars.len() && is_small_y(chars[i + 1]) {
            morae.push(format!("{}{}", c, chars[i + 1]));
            i += 2;
        } else {
            morae.push(c.to_string());
            i += 1;
        }
    }
    morae
}

/// 단일 모라(촉음/발음/장음 제외)를 로마자로 변환한다.
fn mora_romaji(mora: &str) -> Option<&'static str> {
    Some(match mora {
        "あ" => "a", "い" => "i", "う" => "u", "え" => "e", "お" => "o",
        "か" => "ka", "き" => "ki", "く" => "ku", "け" => "ke", "こ" => "ko",
        "さ" => "sa", "し" => "shi", "す" => "su", "せ" => "se", "そ" => "so",
        "た" => "ta", "ち" => "chi", "つ" => "tsu", "て" => "te", "と" => "to",
        "な" => "na", "に" => "ni", "ぬ" => "nu", "ね" => "ne", "の" => "no",
        "は" => "ha", "ひ" => "hi", "ふ" => "fu", "へ" => "he", "ほ" => "ho",
        "ま" => "ma", "み" => "mi", "む" => "mu", "め" => "me", "も" => "mo",
        "や" => "ya", "ゆ" => "yu", "よ" => "yo",
        "ら" => "ra", "り" => "ri", "る" => "ru", "れ" => "re", "ろ" => "ro",
        "わ" => "wa", "ゐ" => "i", "ゑ" => "e", "を" => "o",
        "が" => "ga", "ぎ" => "gi", "ぐ" => "gu", "げ" => "ge", "ご" => "go",
        "ざ" => "za", "じ" => "ji", "ず" => "zu", "ぜ" => "ze", "ぞ" => "zo",
        "だ" => "da", "ぢ" => "ji", "づ" => "zu", "で" => "de", "ど" => "do",
        "ば" => "ba", "び" => "bi", "ぶ" => "bu", "べ" => "be", "ぼ" => "bo",
        "ぱ" => "pa", "ぴ" => "pi", "ぷ" => "pu", "ぺ" => "pe", "ぽ" => "po",

        "きゃ" => "kya", "きゅ" => "kyu", "きょ" => "kyo",
        "ぎゃ" => "gya", "ぎゅ" => "gyu", "ぎょ" => "gyo",
        "しゃ" => "sha", "しゅ" => "shu", "しょ" => "sho",
        "じゃ" => "ja", "じゅ" => "ju", "じょ" => "jo",
        "ちゃ" => "cha", "ちゅ" => "chu", "ちょ" => "cho",
        "ぢゃ" => "ja", "ぢゅ" => "ju", "ぢょ" => "jo",
        "にゃ" => "nya", "にゅ" => "nyu", "にょ" => "nyo",
        "ひゃ" => "hya", "ひゅ" => "hyu", "ひょ" => "hyo",
        "びゃ" => "bya", "びゅ" => "byu", "びょ" => "byo",
        "ぴゃ" => "pya", "ぴゅ" => "pyu", "ぴょ" => "pyo",
        "みゃ" => "mya", "みゅ" => "myu", "みょ" => "myo",
        "りゃ" => "rya", "りゅ" => "ryu", "りょ" => "ryo",

        _ => return None,
    })
}

/// 촉음(っ) 뒤에 오는 로마자의 자음을 겹쳐 쓴다. 단, ch로 시작하면
/// 헵번식 관례에 따라 t를 덧붙인다 (예: っち → "tchi").
fn double_leading_consonant(next_romaji: &str) -> String {
    if next_romaji.starts_with("ch") {
        format!("t{next_romaji}")
    } else if let Some(first) = next_romaji.chars().next() {
        if "aiueo".contains(first) {
            // 모음으로 시작하면 겹칠 자음이 없다 (비정상 입력) — 그대로 둔다.
            next_romaji.to_string()
        } else {
            format!("{first}{next_romaji}")
        }
    } else {
        String::new()
    }
}

fn morae_to_romaji(morae: &[String]) -> String {
    let mut out = String::new();
    let mut i = 0;
    // っ는 그 자체로는 소리를 내지 않고, 다음 모라의 자음을 겹치게 만드는
    // 표시이므로 플래그로 다음 반복까지 전달한다.
    let mut geminate = false;
    while i < morae.len() {
        let m = morae[i].as_str();
        match m {
            "っ" => {
                geminate = true;
            }
            "ん" => {
                let next = morae.get(i + 1).and_then(|n| mora_romaji(n));
                match next {
                    Some(nr) if nr.starts_with(|c: char| "aiueoy".contains(c)) => {
                        out.push_str("n'");
                    }
                    Some(nr)
                        if nr.starts_with('b') || nr.starts_with('m') || nr.starts_with('p') =>
                    {
                        out.push('m');
                    }
                    _ => out.push('n'),
                }
                geminate = false;
            }
            "ー" => {
                if let Some(last_vowel) = out.chars().rev().find(|c| "aiueo".contains(*c)) {
                    out.push(last_vowel);
                }
                geminate = false;
            }
            other => {
                if let Some(r) = mora_romaji(other) {
                    if geminate {
                        out.push_str(&double_leading_consonant(r));
                    } else {
                        out.push_str(r);
                    }
                } else {
                    // 가나가 아닌 문자(구두점 등)는 그대로 통과시킨다.
                    out.push_str(other);
                }
                geminate = false;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_plain_hiragana() {
        assert_eq!(kana_to_romaji("まなぶ"), "manabu");
    }

    #[test]
    fn converts_plain_katakana() {
        assert_eq!(kana_to_romaji("ガク"), "gaku");
    }

    #[test]
    fn converts_youon() {
        assert_eq!(kana_to_romaji("きゃく"), "kyaku");
        assert_eq!(kana_to_romaji("キャク"), "kyaku");
    }

    #[test]
    fn converts_sokuon() {
        assert_eq!(kana_to_romaji("がっこう"), "gakkou");
        assert_eq!(kana_to_romaji("マッチ"), "matchi");
    }

    #[test]
    fn converts_choonpu() {
        assert_eq!(kana_to_romaji("コーヒー"), "koohii");
    }

    #[test]
    fn converts_n_before_labial() {
        // ん + ば行 → "m" (전통 헵번식)
        assert_eq!(kana_to_romaji("しんぶん"), "shimbun");
    }

    #[test]
    fn converts_n_before_vowel_with_apostrophe() {
        assert_eq!(kana_to_romaji("けんい"), "ken'i");
    }

    #[test]
    fn converts_n_at_end() {
        assert_eq!(kana_to_romaji("ほん"), "hon");
    }
}

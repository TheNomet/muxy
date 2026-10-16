use crate::voice::Language;

const LANGUAGES: &[(&str, &str)] = &[
    ("af", "Afrikaans"),
    ("sq", "Albanian"),
    ("am", "Amharic"),
    ("ar", "Arabic"),
    ("hy", "Armenian"),
    ("as", "Assamese"),
    ("az", "Azerbaijani"),
    ("ba", "Bashkir"),
    ("eu", "Basque"),
    ("be", "Belarusian"),
    ("bn", "Bengali"),
    ("bs", "Bosnian"),
    ("br", "Breton"),
    ("bg", "Bulgarian"),
    ("my", "Burmese"),
    ("ca", "Catalan"),
    ("zh", "Chinese"),
    ("hr", "Croatian"),
    ("cs", "Czech"),
    ("da", "Danish"),
    ("nl", "Dutch"),
    ("en", "English"),
    ("et", "Estonian"),
    ("fo", "Faroese"),
    ("fi", "Finnish"),
    ("fr", "French"),
    ("gl", "Galician"),
    ("ka", "Georgian"),
    ("de", "German"),
    ("el", "Greek"),
    ("gu", "Gujarati"),
    ("ht", "Haitian Creole"),
    ("ha", "Hausa"),
    ("haw", "Hawaiian"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("hu", "Hungarian"),
    ("is", "Icelandic"),
    ("id", "Indonesian"),
    ("it", "Italian"),
    ("ja", "Japanese"),
    ("jw", "Javanese"),
    ("kn", "Kannada"),
    ("kk", "Kazakh"),
    ("km", "Khmer"),
    ("ko", "Korean"),
    ("lo", "Lao"),
    ("la", "Latin"),
    ("lv", "Latvian"),
    ("ln", "Lingala"),
    ("lt", "Lithuanian"),
    ("lb", "Luxembourgish"),
    ("mk", "Macedonian"),
    ("mg", "Malagasy"),
    ("ms", "Malay"),
    ("ml", "Malayalam"),
    ("mt", "Maltese"),
    ("mi", "Maori"),
    ("mr", "Marathi"),
    ("mn", "Mongolian"),
    ("ne", "Nepali"),
    ("no", "Norwegian"),
    ("nn", "Nynorsk"),
    ("oc", "Occitan"),
    ("ps", "Pashto"),
    ("fa", "Persian"),
    ("pl", "Polish"),
    ("pt", "Portuguese"),
    ("pa", "Punjabi"),
    ("ro", "Romanian"),
    ("ru", "Russian"),
    ("sa", "Sanskrit"),
    ("sr", "Serbian"),
    ("sn", "Shona"),
    ("sd", "Sindhi"),
    ("si", "Sinhala"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("so", "Somali"),
    ("es", "Spanish"),
    ("su", "Sundanese"),
    ("sw", "Swahili"),
    ("sv", "Swedish"),
    ("tl", "Tagalog"),
    ("tg", "Tajik"),
    ("ta", "Tamil"),
    ("tt", "Tatar"),
    ("te", "Telugu"),
    ("th", "Thai"),
    ("bo", "Tibetan"),
    ("tr", "Turkish"),
    ("tk", "Turkmen"),
    ("uk", "Ukrainian"),
    ("ur", "Urdu"),
    ("uz", "Uzbek"),
    ("vi", "Vietnamese"),
    ("cy", "Welsh"),
    ("yi", "Yiddish"),
    ("yo", "Yoruba"),
];

pub fn languages() -> Vec<Language> {
    let preferred = system();
    LANGUAGES
        .iter()
        .map(|(id, name)| Language {
            id: (*id).into(),
            name: (*name).into(),
            preferred: *id == preferred,
        })
        .collect()
}

pub(super) fn resolve(requested: &str) -> Result<String, String> {
    if requested.is_empty() {
        return Ok(system());
    }
    normalize(requested).ok_or_else(|| {
        crate::tr!("Choose a supported dictation language in Settings → Composer.").into()
    })
}

fn system() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()))
        .and_then(|value| normalize(&value))
        .unwrap_or_else(|| "en".into())
}

fn normalize(locale: &str) -> Option<String> {
    let base = locale
        .split(['-', '_', '.', '@'])
        .next()?
        .to_ascii_lowercase();
    let base = match base.as_str() {
        "nb" => "no",
        "iw" => "he",
        "jv" => "jw",
        _ => &base,
    };
    LANGUAGES
        .iter()
        .any(|(id, _)| *id == base)
        .then(|| base.to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn imported_locales_keep_the_language_without_becoming_cli_options() {
        assert_eq!(super::resolve("de-DE"), Ok("de".into()));
        assert_eq!(super::resolve("zh_Hant_TW"), Ok("zh".into()));
        assert_eq!(super::resolve("nb_NO.UTF-8"), Ok("no".into()));
        assert!(super::resolve("--translate").is_err());
        assert!(super::resolve("unknown").is_err());
    }
}

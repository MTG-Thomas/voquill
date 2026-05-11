const GLOSSARY_TERMS: &[&str] = &[
    "Microsoft 365",
    "Entra ID",
    "Intune",
    "Defender for Endpoint",
    "Defender XDR",
    "Microsoft Lighthouse",
    "Exchange Online",
    "SharePoint",
    "OneDrive",
    "Microsoft Teams",
    "Azure",
    "Azure OpenAI",
    "Autotask",
    "HaloPSA",
    "NinjaOne",
    "Keeper",
    "BitLocker",
    "DNS",
    "DHCP",
    "RMM",
    "PSA",
    "MFA",
    "SSO",
    "SAML",
    "OAuth",
    "OIDC",
    "SMTP",
    "IMAP",
    "MX",
    "SPF",
    "DKIM",
    "DMARC",
    "CIPP",
    "Bifrost",
    "Midtown Technology Group",
];

const CORRECTIONS: &[(&str, &str)] = &[
    ("in tune", "Intune"),
    ("in toon", "Intune"),
    ("entra i d", "Entra ID"),
    ("entra id", "Entra ID"),
    ("entry id", "Entra ID"),
    ("defender x d r", "Defender XDR"),
    ("defender xdr", "Defender XDR"),
    ("auto task", "Autotask"),
    ("halo p s a", "HaloPSA"),
    ("halo psa", "HaloPSA"),
    ("ninja one", "NinjaOne"),
    ("bit locker", "BitLocker"),
    ("one drive", "OneDrive"),
    ("share point", "SharePoint"),
    ("microsoft team", "Microsoft Teams"),
    ("azure open ai", "Azure OpenAI"),
    ("open ai", "OpenAI"),
    ("d mark", "DMARC"),
    ("dmarc", "DMARC"),
    ("d kim", "DKIM"),
    ("dkim", "DKIM"),
    ("s p f", "SPF"),
    ("spf", "SPF"),
    ("m f a", "MFA"),
    ("s s o", "SSO"),
    ("saml", "SAML"),
    ("o auth", "OAuth"),
    ("oath", "OAuth"),
    ("o i d c", "OIDC"),
    ("smtp", "SMTP"),
    ("imap", "IMAP"),
    ("r m m", "RMM"),
    ("p s a", "PSA"),
    ("c i p p", "CIPP"),
];

pub fn build_transcription_prompt(language_hint: Option<&str>, custom_vocabulary: &str) -> String {
    let mut prompt = String::new();
    if let Some(hint) = language_hint {
        let trimmed = hint.trim();
        if !trimmed.is_empty() {
            prompt.push_str(trimmed);
            if !trimmed.ends_with('.') {
                prompt.push('.');
            }
            prompt.push(' ');
        }
    }

    let mut terms = GLOSSARY_TERMS
        .iter()
        .map(|term| (*term).to_string())
        .collect::<Vec<_>>();
    terms.extend(parse_custom_vocabulary_terms(custom_vocabulary));

    prompt.push_str("Prefer these IT and MSP terms when they match the spoken audio: ");
    prompt.push_str(&terms.join(", "));
    prompt.push('.');
    prompt
}

pub fn correct_transcription(text: &str, custom_corrections: &str) -> String {
    let corrected =
        CORRECTIONS
            .iter()
            .fold(text.to_string(), |corrected, (phrase, replacement)| {
                replace_phrase_case_insensitive(&corrected, phrase, replacement)
            });

    parse_custom_correction_pairs(custom_corrections)
        .iter()
        .fold(corrected, |corrected, (phrase, replacement)| {
            replace_phrase_case_insensitive(&corrected, phrase, replacement)
        })
}

fn parse_custom_vocabulary_terms(custom_vocabulary: &str) -> Vec<String> {
    custom_vocabulary
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToString::to_string)
        .collect()
}

fn parse_custom_correction_pairs(custom_corrections: &str) -> Vec<(String, String)> {
    custom_corrections
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }

            let (from, to) = trimmed
                .split_once("=>")
                .or_else(|| trimmed.split_once("->"))?;
            let from = from.trim();
            let to = to.trim();
            if from.is_empty() || to.is_empty() {
                return None;
            }

            Some((from.to_string(), to.to_string()))
        })
        .collect()
}

fn replace_phrase_case_insensitive(text: &str, phrase: &str, replacement: &str) -> String {
    let lower_text = text.to_ascii_lowercase();
    let lower_phrase = phrase.to_ascii_lowercase();
    let mut output = String::with_capacity(text.len());
    let mut search_start = 0;
    let mut copied_until = 0;

    while let Some(relative_match_start) = lower_text[search_start..].find(&lower_phrase) {
        let match_start = search_start + relative_match_start;
        let match_end = match_start + lower_phrase.len();

        if is_phrase_boundary(text, match_start, match_end) {
            output.push_str(&text[copied_until..match_start]);
            output.push_str(replacement);
            copied_until = match_end;
        }

        search_start = match_end;
    }

    output.push_str(&text[copied_until..]);
    output
}

fn is_phrase_boundary(text: &str, start: usize, end: usize) -> bool {
    is_boundary_before(text, start) && is_boundary_after(text, end)
}

fn is_boundary_before(text: &str, start: usize) -> bool {
    if start == 0 {
        return true;
    }

    text[..start]
        .chars()
        .next_back()
        .map(is_word_separator)
        .unwrap_or(true)
}

fn is_boundary_after(text: &str, end: usize) -> bool {
    if end >= text.len() {
        return true;
    }

    text[end..]
        .chars()
        .next()
        .map(is_word_separator)
        .unwrap_or(true)
}

fn is_word_separator(character: char) -> bool {
    !character.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{build_transcription_prompt, correct_transcription};

    #[test]
    fn prompt_includes_language_hint_and_it_terms() {
        let prompt = build_transcription_prompt(Some("American spelling"), "");

        assert!(prompt.starts_with("American spelling."));
        assert!(prompt.contains("Entra ID"));
        assert!(prompt.contains("HaloPSA"));
        assert!(prompt.contains("DMARC"));
    }

    #[test]
    fn prompt_handles_missing_language_hint() {
        let prompt = build_transcription_prompt(None, "");

        assert!(prompt.starts_with("Prefer these IT and MSP terms"));
    }

    #[test]
    fn corrects_high_confidence_it_terms() {
        let text = "Please check in tune, halo p s a, ninja one, and d mark records.";

        assert_eq!(
            correct_transcription(text, ""),
            "Please check Intune, HaloPSA, NinjaOne, and DMARC records."
        );
    }

    #[test]
    fn does_not_rewrite_inside_larger_words() {
        let text = "This open aim sentence mentions spfing and autodmark.";

        assert_eq!(correct_transcription(text, ""), text);
    }

    #[test]
    fn keeps_replacements_case_stable() {
        let text = "ENTRA ID and defender x d r should be canonical.";

        assert_eq!(
            correct_transcription(text, ""),
            "Entra ID and Defender XDR should be canonical."
        );
    }

    #[test]
    fn includes_custom_vocabulary_terms() {
        let prompt = build_transcription_prompt(None, "Contoso Dental\n# comment\nGraphConnector");

        assert!(prompt.contains("Contoso Dental"));
        assert!(prompt.contains("GraphConnector"));
        assert!(!prompt.contains("# comment"));
    }

    #[test]
    fn applies_custom_correction_pairs() {
        let text = "Please check contoso dent all and graph connector.";
        let corrections = "contoso dent all => Contoso Dental\ngraph connector -> GraphConnector";

        assert_eq!(
            correct_transcription(text, corrections),
            "Please check Contoso Dental and GraphConnector."
        );
    }
}

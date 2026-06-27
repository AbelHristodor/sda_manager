/// Lowercase a string and replace Romanian (and common) diacritics with their
/// ASCII base letters, so searches are accent-insensitive.
pub fn fold(input: &str) -> String {
    input
        .chars()
        .flat_map(|c| {
            let mapped = match c {
                'ă' | 'â' | 'à' | 'á' | 'Ă' | 'Â' => 'a',
                'î' | 'í' | 'ì' | 'Î' => 'i',
                'ș' | 'ş' | 'Ș' | 'Ş' => 's',
                'ț' | 'ţ' | 'Ț' | 'Ţ' => 't',
                'é' | 'è' | 'ê' | 'É' => 'e',
                'ó' | 'ô' | 'ö' | 'Ó' => 'o',
                'ú' | 'ü' | 'û' | 'Ú' => 'u',
                other => other,
            };
            mapped.to_lowercase()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn folds_romanian_diacritics_and_lowercases() {
        assert_eq!(fold("Plecaţi-vă"), "plecati-va");
        assert_eq!(fold("Cunoaşteţi"), "cunoasteti");
        assert_eq!(fold("ÎNÂ ȘȚ"), "ina st");
    }

    #[test]
    fn leaves_plain_ascii_untouched() {
        assert_eq!(fold("Imnul 150"), "imnul 150");
    }
}

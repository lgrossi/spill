pub(crate) fn column_key(title: &str, position: usize) -> String {
    let slug = title
        .chars()
        .filter_map(|character| {
            if character.is_ascii_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character.is_whitespace() || character == '-' || character == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();

    if slug.is_empty() {
        format!("column_{position}")
    } else {
        format!("{position}_{slug}")
    }
}

pub(crate) fn column_accent_color(title: &str, position: usize) -> &'static str {
    let title = title.to_lowercase();
    if title.contains("action") {
        "#8757b6"
    } else if title.contains("well") || title.contains("liked") {
        "#2f9469"
    } else if title.contains("wrong") || title.contains("lacked") || title.contains("improve") {
        "#cf4f4f"
    } else if title.contains("learned") {
        "#0f5f72"
    } else if title.contains("longed") {
        "#cf8a3f"
    } else if title.contains("feeling") {
        "#0f5f72"
    } else if title.contains("mood") {
        "#cf8a3f"
    } else {
        ["#cf8a3f", "#2f9469", "#cf4f4f", "#8757b6"][position % 4]
    }
}

pub(crate) fn cluster_key(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .find(|word| word.len() >= 4)
}

pub(crate) fn manual_cluster_title(source: &str, target: &str) -> String {
    let key = cluster_key(source)
        .or_else(|| cluster_key(target))
        .unwrap_or_else(|| "cards".to_owned());
    format!("Grouped: {key}")
}

pub(crate) fn action_tags(text: &str) -> Vec<String> {
    let mut tags = text
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|word| word.len() >= 4)
        .take(3)
        .collect::<Vec<_>>();
    if !tags.iter().any(|tag| tag == "topvoted") {
        tags.push("topvoted".to_owned());
    }
    tags
}

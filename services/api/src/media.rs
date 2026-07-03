use std::{collections::HashMap, env};

use axum::{Json, extract::Query};
use serde::Deserialize;

use crate::contracts::{GifResult, GifSearchQuery, GifSearchResponse};

pub async fn search_gifs(Query(query): Query<GifSearchQuery>) -> Json<GifSearchResponse> {
    let provider = GifProvider;
    match provider
        .search(
            query.q.as_deref().unwrap_or_default(),
            query.page,
            MediaSearchKind::from_query(query.kind.as_deref()),
        )
        .await
    {
        Ok(results) => Json(GifSearchResponse {
            results,
            degraded: false,
        }),
        Err(()) => Json(GifSearchResponse {
            results: Vec::new(),
            degraded: true,
        }),
    }
}

struct GifProvider;

impl GifProvider {
    async fn search(
        &self,
        query: &str,
        page: usize,
        kind: MediaSearchKind,
    ) -> Result<Vec<GifResult>, ()> {
        let query = query.trim();
        if query.eq_ignore_ascii_case("fail") {
            return Err(());
        }
        if query.is_empty() {
            return Ok(Vec::new());
        }

        match search_klipy(query, page, kind).await {
            Ok(results) if !results.is_empty() => Ok(results),
            Ok(_) | Err(()) => Err(()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MediaSearchKind {
    All,
    Gif,
    Sticker,
    Clip,
}

impl MediaSearchKind {
    fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("gif") => Self::Gif,
            Some("sticker") => Self::Sticker,
            Some("clip") => Self::Clip,
            _ => Self::All,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Gif => "gif",
            Self::Sticker => "sticker",
            Self::Clip => "clip",
        }
    }
}

async fn search_klipy(
    query: &str,
    page: usize,
    kind: MediaSearchKind,
) -> Result<Vec<GifResult>, ()> {
    match kind {
        MediaSearchKind::Sticker => search_klipy_typed(query, page, "stickers", kind).await,
        MediaSearchKind::All | MediaSearchKind::Gif | MediaSearchKind::Clip => {
            search_klipy_unified(query, page, kind).await
        }
    }
}

async fn search_klipy_unified(
    query: &str,
    page: usize,
    kind: MediaSearchKind,
) -> Result<Vec<GifResult>, ()> {
    let api_key = env::var("SPILLIO_KLIPY_API_KEY").map_err(|_| ())?;
    let client = reqwest::Client::new();
    let mut pos: Option<String> = None;
    let mut response = None;

    for _ in 0..=page {
        let mut request = client.get("https://api.klipy.com/v2/search").query(&[
            ("q", query.to_owned()),
            ("key", api_key.clone()),
            ("limit", "8".to_owned()),
        ]);
        if let Some(pos) = &pos {
            request = request.query(&[("pos", pos)]);
        }
        let search = request.send().await.map_err(|_| ())?;
        if !search.status().is_success() {
            return Err(());
        }
        let search = search.json::<KlipySearchResponse>().await.map_err(|_| ())?;
        pos = search.next.clone();
        response = Some(search);
    }

    Ok(response
        .ok_or(())?
        .results
        .into_iter()
        .filter_map(|result| {
            let formats = &result.media_formats;
            let image =
                select_klipy_media(formats, &["gif", "mediumgif", "tinygif", "nanogif", "webp"]);
            let video = select_klipy_media(
                formats,
                &["mp4", "webm", "loopedmp4", "tinymp4", "tinywebm"],
            );
            let media = if kind == MediaSearchKind::Clip {
                video.or(image)
            } else {
                image.or(video)
            }?;
            // Klipy's "*preview" formats (gifpreview, preview, tinygifpreview, ...) are
            // static JPG poster frames despite the name -- picking them made search
            // thumbnails and the composer's selected-GIF preview render as static images.
            // Use small *animated* formats instead so previews actually loop.
            let preview =
                select_klipy_media(formats, &["nanogif", "tinygif", "webp", "mediumgif", "gif"])
                    .map(|media| media.url.clone())
                    .unwrap_or_else(|| media.url.clone());
            Some(GifResult {
                id: format!("klipy-{}", result.id),
                url: media.url.clone(),
                preview_url: preview,
                alt_text: if result.title.trim().is_empty() {
                    format!("{query} GIF")
                } else {
                    result.title
                },
                media_type: if kind == MediaSearchKind::Clip && video.is_some() {
                    "video"
                } else {
                    "image"
                }
                .to_owned(),
                kind: kind.as_str().to_owned(),
            })
        })
        .collect())
}

async fn search_klipy_typed(
    query: &str,
    page: usize,
    endpoint: &str,
    kind: MediaSearchKind,
) -> Result<Vec<GifResult>, ()> {
    let api_key = env::var("SPILLIO_KLIPY_API_KEY").map_err(|_| ())?;
    let response = reqwest::Client::new()
        .get(format!("https://api.klipy.com/v2/{endpoint}/search"))
        .query(&[
            ("q", query.to_owned()),
            ("key", api_key),
            ("limit", "8".to_owned()),
            ("offset", (page * 8).to_string()),
        ])
        .send()
        .await
        .map_err(|_| ())?;
    if !response.status().is_success() {
        return Err(());
    }
    let response = response
        .json::<KlipyTypedSearchResponse>()
        .await
        .map_err(|_| ())?;
    Ok(response
        .data
        .into_iter()
        .filter_map(|result| {
            let original = result.images.original?;
            let url = original.fallback_url.clone();
            Some(GifResult {
                id: format!("klipy-{}", result.id),
                url: url.clone(),
                preview_url: original.webp.or(original.mp4).unwrap_or(url),
                alt_text: if result.title.trim().is_empty() {
                    format!("{query} {}", kind.as_str())
                } else {
                    result.title
                },
                media_type: "image".to_owned(),
                kind: kind.as_str().to_owned(),
            })
        })
        .collect())
}

#[derive(Deserialize)]
struct KlipySearchResponse {
    #[serde(default)]
    results: Vec<KlipyResult>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct KlipyResult {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    media_formats: HashMap<String, KlipyMedia>,
}

#[derive(Deserialize)]
struct KlipyMedia {
    url: String,
}

#[derive(Deserialize)]
struct KlipyTypedSearchResponse {
    #[serde(default)]
    data: Vec<KlipyTypedResult>,
}

#[derive(Deserialize)]
struct KlipyTypedResult {
    id: String,
    #[serde(default)]
    title: String,
    images: KlipyImages,
}

#[derive(Deserialize)]
struct KlipyImages {
    original: Option<KlipyOriginalImage>,
}

#[derive(Deserialize)]
struct KlipyOriginalImage {
    #[serde(rename = "url")]
    fallback_url: String,
    webp: Option<String>,
    mp4: Option<String>,
}

fn select_klipy_media<'a>(
    formats: &'a HashMap<String, KlipyMedia>,
    preferred_formats: &[&str],
) -> Option<&'a KlipyMedia> {
    preferred_formats
        .iter()
        .find_map(|format| formats.get(*format))
        .or_else(|| formats.values().next())
}

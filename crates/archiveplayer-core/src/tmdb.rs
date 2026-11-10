use crate::error::{CoreError, CoreResult};
use crate::settings::SettingsStore;
use crate::title_cleaner::clean_for_search;
use futures::future::join_all;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const API_BASE: &str = "https://api.themoviedb.org/3";
const IMAGE_BASE: &str = "https://image.tmdb.org/t/p/w500";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmdbMatch {
    pub tmdb_id: i64,
    pub title: String,
    pub poster_url: Option<String>,
    pub media_type: String,
}

#[derive(Debug, Clone)]
pub struct TmdbEpisodeInfo {
    pub season_number: i32,
    pub episode_number: i32,
    pub name: Option<String>,
    pub overview: Option<String>,
    pub still_url: Option<String>,
}

#[derive(Clone)]
pub struct TmdbClient {
    http: reqwest::Client,
    settings: Arc<SettingsStore>,
    cache: Arc<Mutex<TmdbCache>>,
}

#[derive(Default)]
struct TmdbCache {
    matches: HashMap<String, Option<TmdbMatch>>,
    episodes: HashMap<i64, Vec<TmdbEpisodeInfo>>,
    genres: HashMap<(i64, String), Vec<String>>,
}

impl TmdbClient {
    pub fn new(settings: Arc<SettingsStore>) -> CoreResult<Self> {
        let http = reqwest::Client::builder()
            .user_agent("ArchivePlayer/0.1 (Tauri desktop)")
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            http,
            settings,
            cache: Arc::new(Mutex::new(TmdbCache::default())),
        })
    }

    pub fn is_configured(&self) -> bool {
        self.settings.tmdb_api_key().is_some()
    }

    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.matches.clear();
            cache.episodes.clear();
            cache.genres.clear();
        }
    }

    pub async fn validate_key(&self) -> CoreResult<()> {
        let key = self.api_key()?;
        let response = self
            .http
            .get(format!("{API_BASE}/configuration"))
            .query(&[("api_key", key.as_str())])
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => Ok(()),
            reqwest::StatusCode::UNAUTHORIZED => Err(CoreError::Invalid(
                "TMDB rejected that API key. Double-check the v3 key and try again.".into(),
            )),
            status => Err(CoreError::Message(format!(
                "TMDB returned HTTP {status} while testing the key."
            ))),
        }
    }

    pub async fn find_match(&self, title: &str) -> Option<TmdbMatch> {
        self.find_match_preferring(title, None).await
    }

    /// Matches TMDB metadata, preferring the media type reported by
    /// Archive.org so movies with a same-named TV show resolve correctly.
    pub async fn find_match_for(&self, title: &str, media_type: &str) -> Option<TmdbMatch> {
        let preferred = if media_type.eq_ignore_ascii_case("movies") {
            Some("movie")
        } else {
            Some("tv")
        };
        self.find_match_preferring(title, preferred).await
    }

    async fn find_match_preferring(
        &self,
        title: &str,
        preferred: Option<&str>,
    ) -> Option<TmdbMatch> {
        let cleaned = clean_for_search(title);
        if cleaned.is_empty() {
            return None;
        }
        let cache_key = format!("{}|{}", cleaned.to_lowercase(), preferred.unwrap_or("tv"));

        if let Ok(cache) = self.cache.lock()
            && let Some(cached) = cache.matches.get(&cache_key)
        {
            return cached.clone();
        }

        let result = match self.find_match_uncached(&cleaned, preferred).await {
            Ok(result) => result,
            Err(error) => {
                eprintln!("[tmdb] match lookup failed: {error}");
                None
            }
        };

        if let Ok(mut cache) = self.cache.lock() {
            cache.matches.insert(cache_key, result.clone());
        }

        result
    }

    async fn find_match_uncached(
        &self,
        query: &str,
        preferred: Option<&str>,
    ) -> CoreResult<Option<TmdbMatch>> {
        let order = if preferred == Some("movie") {
            ["movie", "tv"]
        } else {
            ["tv", "movie"]
        };
        for media_type in order {
            if let Some(result) = self.search_one(query, media_type).await? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    async fn search_one(&self, query: &str, media_type: &str) -> CoreResult<Option<TmdbMatch>> {
        let key = self.api_key()?;
        let response: TmdbSearchResponse = self
            .get_json(
                format!("{API_BASE}/search/{media_type}"),
                &[
                    ("api_key", key.as_str()),
                    ("query", query),
                    ("include_adult", "false"),
                ],
            )
            .await?;

        let Some(best) = response.results.into_iter().next() else {
            return Ok(None);
        };

        Ok(Some(TmdbMatch {
            tmdb_id: best.id,
            title: best
                .name
                .or(best.title)
                .unwrap_or_else(|| query.to_string()),
            poster_url: best
                .poster_path
                .filter(|path| !path.is_empty())
                .map(|path| format!("{IMAGE_BASE}{path}")),
            media_type: media_type.to_string(),
        }))
    }

    pub async fn fetch_episodes(&self, tmdb_id: i64) -> CoreResult<Vec<TmdbEpisodeInfo>> {
        if let Ok(cache) = self.cache.lock()
            && let Some(episodes) = cache.episodes.get(&tmdb_id)
        {
            return Ok(episodes.clone());
        }

        let key = self.api_key()?;
        let show: TmdbTvDetails = self
            .get_json(
                format!("{API_BASE}/tv/{tmdb_id}"),
                &[("api_key", key.as_str())],
            )
            .await?;

        let seasons: Vec<i32> = show
            .seasons
            .into_iter()
            .map(|season| season.season_number)
            .filter(|season| *season > 0)
            .collect();

        let futures = seasons.into_iter().map(|season_number| {
            let key = key.clone();
            let http = self.http.clone();
            async move {
                let url = format!("{API_BASE}/tv/{tmdb_id}/season/{season_number}");
                let response = http
                    .get(url)
                    .query(&[("api_key", key.as_str())])
                    .send()
                    .await?
                    .error_for_status()?;
                let details: TmdbSeasonDetails = response.json().await?;
                Ok::<_, CoreError>((season_number, details))
            }
        });

        let mut episodes = Vec::new();
        for result in join_all(futures).await {
            let (season_number, details) = result?;
            for episode in details.episodes {
                episodes.push(TmdbEpisodeInfo {
                    season_number,
                    episode_number: episode.episode_number,
                    name: episode.name,
                    overview: episode.overview.filter(|text| !text.trim().is_empty()),
                    still_url: episode
                        .still_path
                        .filter(|path| !path.is_empty())
                        .map(|path| format!("{IMAGE_BASE}{path}")),
                });
            }
        }

        if let Ok(mut cache) = self.cache.lock() {
            cache.episodes.insert(tmdb_id, episodes.clone());
        }

        Ok(episodes)
    }

    pub async fn fetch_genres(&self, tmdb_id: i64, media_type: &str) -> CoreResult<Vec<String>> {
        let cache_key = (tmdb_id, media_type.to_string());
        if let Ok(cache) = self.cache.lock()
            && let Some(genres) = cache.genres.get(&cache_key)
        {
            return Ok(genres.clone());
        }

        let key = self.api_key()?;
        let details: TmdbDetails = self
            .get_json(
                format!("{API_BASE}/{media_type}/{tmdb_id}"),
                &[("api_key", key.as_str())],
            )
            .await?;

        let genres: Vec<String> = details
            .genres
            .into_iter()
            .filter_map(|genre| genre.name)
            .filter(|name| !name.trim().is_empty())
            .collect();

        if let Ok(mut cache) = self.cache.lock() {
            cache.genres.insert(cache_key, genres.clone());
        }

        Ok(genres)
    }

    fn api_key(&self) -> CoreResult<String> {
        self.settings
            .tmdb_api_key()
            .ok_or_else(|| CoreError::Invalid("TMDB is not configured yet.".into()))
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        url: String,
        query: &[(&str, &str)],
    ) -> CoreResult<T> {
        let response = self.http.get(url).query(query).send().await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(CoreError::Invalid(
                "TMDB rejected the configured API key.".into(),
            ));
        }

        let response = response
            .error_for_status()
            .map_err(|error| CoreError::Message(format!("TMDB request failed: {error}")))?;

        Ok(response.json().await?)
    }
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse {
    #[serde(default)]
    results: Vec<TmdbSearchResult>,
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResult {
    id: i64,
    title: Option<String>,
    name: Option<String>,
    poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvDetails {
    #[serde(default)]
    seasons: Vec<TmdbSeasonSummary>,
}

#[derive(Debug, Deserialize)]
struct TmdbSeasonSummary {
    season_number: i32,
}

#[derive(Debug, Deserialize)]
struct TmdbSeasonDetails {
    #[serde(default)]
    episodes: Vec<TmdbEpisode>,
}

#[derive(Debug, Deserialize)]
struct TmdbEpisode {
    episode_number: i32,
    name: Option<String>,
    still_path: Option<String>,
    overview: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbDetails {
    #[serde(default)]
    genres: Vec<TmdbGenre>,
}

#[derive(Debug, Deserialize)]
struct TmdbGenre {
    name: Option<String>,
}

use crate::archive::ArchiveClient;
use crate::db::{Database, MediaItemRow, NewMediaItem};
use crate::episodes::{match_episode_to_file, parse_episodes_from_files};
use crate::error::{CoreError, CoreResult};
use crate::models::{
    ArchiveMetadata, CategorySuggestion, Episode, MediaDetails, SettingsInfo, UploadedMedia,
    VideoFileUrl,
};
use crate::playback;
use crate::settings::SettingsStore;
use crate::tmdb::TmdbClient;
use crate::util::parse_archive_identifier;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Application services shared by the Tauri commands.
pub struct Service {
    pub db: Arc<Database>,
    pub archive: ArchiveClient,
    pub tmdb: TmdbClient,
    pub settings: Arc<SettingsStore>,
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
}

impl Service {
    pub fn new(data_dir: PathBuf) -> CoreResult<Self> {
        std::fs::create_dir_all(&data_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o700));
        }
        let settings_path = data_dir.join("settings.json");
        let database_path = data_dir.join("archiveplayer.db");

        let settings = Arc::new(SettingsStore::load(settings_path)?);
        let db = Arc::new(Database::open(&database_path)?);
        let archive = ArchiveClient::new()?;
        let tmdb = TmdbClient::new(settings.clone())?;

        Ok(Self {
            db,
            archive,
            tmdb,
            settings,
            data_dir,
            database_path,
        })
    }

    pub fn settings_info(
        &self,
        app_version: &str,
        runtime: crate::models::RuntimeMode,
        can_manage_settings: bool,
    ) -> SettingsInfo {
        let settings = self.settings.get();
        SettingsInfo {
            tmdb_api_key: settings.tmdb_api_key.clone(),
            tmdb_configured: settings.usable_tmdb_key().is_some(),
            last_email: settings.last_email.clone(),
            data_dir: self.data_dir.display().to_string(),
            settings_path: self.settings.path().display().to_string(),
            database_path: self.database_path.display().to_string(),
            app_version: app_version.to_string(),
            can_manage_settings,
            runtime,
        }
    }

    pub async fn add_to_library(
        &self,
        user_id: i64,
        input: &str,
        category_name: &str,
    ) -> CoreResult<crate::models::LibraryItem> {
        let identifier = parse_archive_identifier(input)?;
        let metadata = self.archive.metadata(&identifier).await?;

        if metadata.video_files.is_empty() {
            return Err(CoreError::Invalid(
                "That Archive.org item has no playable video files.".into(),
            ));
        }

        let media = match self.db.media_by_identifier(&identifier)? {
            Some(existing) => self.upgrade_media_if_needed(existing, &metadata).await?,
            None => self.create_media(&metadata).await?,
        };

        self.db.add_user_media(user_id, media.id, category_name)
    }

    async fn create_media(&self, metadata: &ArchiveMetadata) -> CoreResult<MediaItemRow> {
        let tmdb_match = if self.tmdb.is_configured() {
            self.tmdb
                .find_match_for(&metadata.title, &metadata.media_type)
                .await
        } else {
            None
        };
        let archive_thumbnail = format!("https://archive.org/services/img/{}", metadata.identifier);

        let title = tmdb_match
            .as_ref()
            .map(|matched| matched.title.clone())
            .unwrap_or_else(|| metadata.title.clone());
        let tmdb_poster_url = tmdb_match
            .as_ref()
            .and_then(|matched| matched.poster_url.clone());
        let tmdb_backdrop_url = tmdb_match
            .as_ref()
            .and_then(|matched| matched.backdrop_url.clone());
        let thumbnail_url = tmdb_poster_url.clone().unwrap_or(archive_thumbnail);

        self.db.insert_media(&NewMediaItem {
            identifier: metadata.identifier.clone(),
            title,
            description: metadata.description.clone(),
            thumbnail_url: Some(thumbnail_url),
            media_type: metadata.media_type.clone(),
            tmdb_poster_url,
            tmdb_backdrop_url,
            tmdb_id: tmdb_match.as_ref().map(|matched| matched.tmdb_id),
            tmdb_media_type: tmdb_match
                .as_ref()
                .map(|matched| matched.media_type.clone()),
        })
    }

    async fn upgrade_media_if_needed(
        &self,
        mut media: MediaItemRow,
        metadata: &ArchiveMetadata,
    ) -> CoreResult<MediaItemRow> {
        let needs_upgrade = media.tmdb_poster_url.is_none()
            || media.tmdb_backdrop_url.is_none()
            || media.tmdb_id.is_none()
            || is_messy_title(&media.title)
            || media.description.is_none();

        if !needs_upgrade {
            return Ok(media);
        }

        if media.description.is_none() {
            media.description = metadata.description.clone();
        }
        if media.thumbnail_url.is_none() {
            media.thumbnail_url = Some(format!(
                "https://archive.org/services/img/{}",
                metadata.identifier
            ));
        }

        if (media.tmdb_poster_url.is_none()
            || media.tmdb_backdrop_url.is_none()
            || media.tmdb_id.is_none()
            || is_messy_title(&media.title))
            && self.tmdb.is_configured()
            && let Some(matched) = self
                .tmdb
                .find_match_for(&metadata.title, &metadata.media_type)
                .await
        {
            media.title = matched.title;
            media.tmdb_id = Some(matched.tmdb_id);
            media.tmdb_media_type = Some(matched.media_type);
            if let Some(poster) = matched.poster_url {
                media.tmdb_poster_url = Some(poster.clone());
                media.thumbnail_url = Some(poster);
            }
            if let Some(backdrop) = matched.backdrop_url {
                media.tmdb_backdrop_url = Some(backdrop);
            }
        }

        self.db.update_media(&media)?;
        Ok(media)
    }

    pub async fn category_suggestions(
        &self,
        identifier: &str,
    ) -> CoreResult<Vec<CategorySuggestion>> {
        if !self.tmdb.is_configured() {
            return Ok(Vec::new());
        }

        let metadata = match self.archive.metadata(identifier).await {
            Ok(metadata) => metadata,
            Err(error) => {
                eprintln!("[suggestions] could not read metadata: {error}");
                return Ok(Vec::new());
            }
        };

        let Some(matched) = self
            .tmdb
            .find_match_for(&metadata.title, &metadata.media_type)
            .await
        else {
            return Ok(Vec::new());
        };

        let genres = match self
            .tmdb
            .fetch_genres(matched.tmdb_id, &matched.media_type)
            .await
        {
            Ok(genres) => genres,
            Err(error) => {
                eprintln!("[suggestions] could not read genres: {error}");
                return Ok(Vec::new());
            }
        };

        let mut seen = std::collections::HashSet::new();
        Ok(genres
            .into_iter()
            .filter(|name| seen.insert(name.to_lowercase()))
            .map(|name| CategorySuggestion {
                name,
                source: "TMDB".into(),
            })
            .collect())
    }

    pub async fn playback_details(
        &self,
        user_id: i64,
        identifier: &str,
        platform: Option<&str>,
        requested_file: Option<&str>,
    ) -> CoreResult<MediaDetails> {
        let media = self
            .db
            .media_by_identifier(identifier)?
            .ok_or_else(|| CoreError::NotFound("Media item not found.".into()))?;

        let metadata = self.archive.metadata(identifier).await?;
        let user_media = self.db.user_media_for_identifier(user_id, identifier)?;

        let mut episodes = if media.tmdb_media_type.as_deref() == Some("tv") {
            if let Some(tmdb_id) = media.tmdb_id {
                match self.tmdb.fetch_episodes(tmdb_id).await {
                    Ok(tmdb_episodes) => {
                        let mut matched = Vec::new();
                        for episode in tmdb_episodes {
                            if let Some(file_name) = match_episode_to_file(
                                episode.season_number,
                                episode.episode_number,
                                episode.name.as_deref(),
                                &metadata.video_files,
                            ) {
                                matched.push(Episode {
                                    season_number: Some(episode.season_number),
                                    episode_number: Some(episode.episode_number),
                                    name: episode.name,
                                    overview: episode.overview,
                                    still_url: episode.still_url,
                                    file_name,
                                });
                            }
                        }
                        matched.sort_by_key(|episode| {
                            (
                                episode.season_number.unwrap_or(i32::MAX),
                                episode.episode_number.unwrap_or(i32::MAX),
                            )
                        });
                        matched
                    }
                    Err(error) => {
                        eprintln!("[playback] could not load TMDB episodes: {error}");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if episodes.is_empty() {
            episodes = parse_episodes_from_files(&metadata.video_files);
        }

        let episode_progress = match &user_media {
            Some(item) => self.db.episode_progress(item.id)?,
            None => Vec::new(),
        };

        let video_file_urls: Vec<VideoFileUrl> = if metadata.video_file_details.is_empty() {
            metadata
                .video_files
                .iter()
                .map(|file_name| VideoFileUrl {
                    file_name: file_name.clone(),
                    url: self.archive.download_url(identifier, file_name),
                    format: None,
                    mime_type: playback::mime_type_for_file(file_name).map(str::to_string),
                    size_bytes: None,
                    length_seconds: None,
                    width: None,
                    height: None,
                    video_codec: None,
                })
                .collect()
        } else {
            metadata
                .video_file_details
                .iter()
                .map(|file| VideoFileUrl {
                    file_name: file.name.clone(),
                    url: self.archive.download_url(identifier, &file.name),
                    format: file.format.clone(),
                    mime_type: playback::mime_type_for_file(&file.name).map(str::to_string),
                    size_bytes: file.size_bytes,
                    length_seconds: file.length_seconds,
                    width: file.width,
                    height: file.height,
                    video_codec: playback::infer_video_codec(file.format.as_deref(), &file.name)
                        .map(str::to_string),
                })
                .collect()
        };
        let playback_plan =
            playback::build_playback_plan(&video_file_urls, platform, requested_file);

        Ok(MediaDetails {
            identifier: media.identifier,
            title: media.title,
            description: media.description.or(metadata.description),
            media_type: media.media_type,
            thumbnail_url: media.thumbnail_url,
            tmdb_poster_url: media.tmdb_poster_url,
            tmdb_backdrop_url: media.tmdb_backdrop_url,
            tmdb_id: media.tmdb_id,
            tmdb_media_type: media.tmdb_media_type,
            video_files: metadata.video_files,
            video_file_urls,
            playback_plan,
            episodes,
            watch_progress_seconds: user_media
                .as_ref()
                .map(|item| item.watch_progress_seconds)
                .unwrap_or_default(),
            duration_seconds: user_media
                .as_ref()
                .map(|item| item.duration_seconds)
                .unwrap_or_default(),
            current_episode_file_name: user_media
                .as_ref()
                .and_then(|item| item.current_episode_file_name.clone()),
            last_watched: user_media
                .as_ref()
                .and_then(|item| item.last_watched.clone()),
            is_completed: user_media
                .as_ref()
                .map(|item| item.is_completed)
                .unwrap_or(false),
            autoplay_enabled: user_media
                .as_ref()
                .map(|item| item.autoplay_enabled)
                .unwrap_or(true),
            library_item_id: user_media.as_ref().map(|item| item.id),
            episode_progress,
        })
    }

    pub fn uploaded_media(&self, metadata: &ArchiveMetadata) -> UploadedMedia {
        UploadedMedia {
            identifier: metadata.identifier.clone(),
            title: metadata.title.clone(),
            media_type: metadata.media_type.clone(),
            description: metadata.description.clone(),
            thumbnail_url: Some(format!(
                "https://archive.org/services/img/{}",
                metadata.identifier
            )),
            tmdb_poster_url: None,
            tmdb_backdrop_url: None,
            tmdb_id: None,
            tmdb_media_type: None,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

fn is_messy_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("dktale") || lower.contains("_202")
}

#[cfg(test)]
mod tests {
    use super::is_messy_title;

    #[test]
    fn detects_messy_titles() {
        assert!(is_messy_title("bakugan-dktale_202604"));
        assert!(!is_messy_title("Bakugan Battle Brawlers"));
    }
}

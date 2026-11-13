-- Wide artwork for TV hero and landscape layouts. Posters stay in
-- tmdb_poster_url; backdrops are a separate 16:9 image from TMDB.
ALTER TABLE media_items ADD COLUMN tmdb_backdrop_url TEXT;

PRAGMA user_version = 4;

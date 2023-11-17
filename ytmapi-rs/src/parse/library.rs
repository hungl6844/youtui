use super::{parse_item_text, ProcessedResult};
use crate::common::library::{LibraryArtist, Playlist};
use crate::common::PlaylistID;
use crate::crawler::JsonCrawler;
use crate::nav_consts::{
    GRID, ITEM_SECTION, MRLIR, MTRIR, NAVIGATION_BROWSE_ID, SECTION_LIST, SECTION_LIST_ITEM,
    SINGLE_COLUMN_TAB, THUMBNAIL_RENDERER, TITLE, TITLE_TEXT,
};
use crate::query::{GetLibraryArtistsQuery, GetLibraryPlaylistQuery};
use crate::{Result, Thumbnail};
use const_format::concatcp;

impl<'a> ProcessedResult<GetLibraryArtistsQuery> {
    // TODO: Continuations
    pub fn parse(self) -> Result<Vec<LibraryArtist>> {
        let ProcessedResult { json_crawler, .. } = self;
        parse_library_artists(json_crawler)
    }
}

impl<'a> ProcessedResult<GetLibraryPlaylistQuery> {
    // TODO: Continuations
    // TODO: Implement count and author fields
    pub fn parse(self) -> Result<Vec<Playlist>> {
        let ProcessedResult { json_crawler, .. } = self;
        parse_library_playlist_query(json_crawler)
    }
}

fn parse_library_artists(mut json_crawler: JsonCrawler) -> Result<Vec<LibraryArtist>> {
    if let Some(contents) = process_library_contents(json_crawler) {
        parse_content_list_artists(contents)
    } else {
        Ok(Vec::new())
    }
}

fn parse_library_playlist_query(mut json_crawler: JsonCrawler) -> Result<Vec<Playlist>> {
    if let Some(contents) = process_library_contents(json_crawler) {
        parse_content_list_playlist(contents)
    } else {
        Ok(Vec::new())
    }
}

// Consider returning ProcessedLibraryContents
fn process_library_contents(mut json_crawler: JsonCrawler) -> Option<JsonCrawler> {
    let section = json_crawler.borrow_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST));
    // Assume empty library in this case.
    if let Ok(section) = section {
        if section.path_exists("itemSectionRenderer") {
            json_crawler
                .navigate_pointer(concatcp!(ITEM_SECTION, GRID))
                .ok()
        } else {
            json_crawler
                .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST_ITEM, GRID))
                .ok()
        }
    } else {
        None
    }
}
fn parse_content_list_artists(mut json_crawler: JsonCrawler) -> Result<Vec<LibraryArtist>> {
    let mut results = Vec::new();
    for result in json_crawler.as_array_iter_mut()? {
        let mut data = result.navigate_pointer(MRLIR)?;
        let channel_id = data.take_value_pointer(NAVIGATION_BROWSE_ID)?;
        let artist = parse_item_text(&mut data, 0, 0)?;
        let byline = parse_item_text(&mut data, 1, 0)?;
        results.push(LibraryArtist {
            channel_id,
            artist,
            byline,
        })
    }
    Ok(results)
}

fn parse_content_list_playlist(mut json_crawler: JsonCrawler) -> Result<Vec<Playlist>> {
    // TODO: Implement count and author fields
    let mut results = Vec::new();
    for result in json_crawler
        .navigate_pointer("/items")?
        .as_array_iter_mut()?
        // First result is just a link to create a new playlist.
        .skip(1)
        .map(|c| c.navigate_pointer(MTRIR))
    {
        let mut result = result?;
        let title = result.take_value_pointer(TITLE_TEXT)?;
        let playlist_id: PlaylistID = result
            .borrow_pointer(concatcp!(TITLE, NAVIGATION_BROWSE_ID))?
            // ytmusicapi uses range index [2:] here but doesn't seem to be required.
            // Revisit later if we crash.
            .take_value()?;
        let thumbnails: Vec<Thumbnail> = result.take_value_pointer(THUMBNAIL_RENDERER)?;
        let mut description = None;
        let count = None;
        let author = None;
        if let Ok(mut subtitle) = result.borrow_pointer("/subtitle") {
            let runs = subtitle.borrow_pointer("/runs")?.into_array_iter_mut()?;
            // Extract description from runs.
            // Collect the iterator of Result<String> into a single Result<String>
            description = Some(
                runs.map(|mut c| c.take_value_pointer::<String, &str>("/text"))
                    .collect::<Result<String>>()?,
            );
        }
        let playlist = Playlist {
            description,
            author,
            playlist_id,
            title,
            thumbnails,
            count,
        };
        results.push(playlist)
    }
    Ok(results)
}

mod tests {
    use serde_json::json;

    use crate::{
        common::{
            library::{LibraryArtist, Playlist},
            PlaylistID, YoutubeID,
        },
        crawler::JsonCrawler,
        parse::ProcessedResult,
        query::{GetLibraryArtistsQuery, GetLibraryPlaylistQuery},
    };

    // Consider if the parse function itself should be removed from impl.
    #[test]
    fn test_library_playlists_dummy_json() {
        let testfile = std::fs::read_to_string("test_json/get_library_playlists.json").unwrap();
        let testfile_json = serde_json::from_str(&testfile).unwrap();
        let json_crawler = JsonCrawler::from_json(testfile_json);
        let processed = ProcessedResult::from_raw(json_crawler, GetLibraryPlaylistQuery {});
        let result = processed.parse().unwrap();
        let expected = json!([
          {
            "playlist_id": "VLLM",
            "title": "Liked Music",
            "thumbnails": [
              {
                "height": 192,
                "width": 192,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/liked-music-@192.png"
              },
              {
                "height": 576,
                "width": 576,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/liked-music-@576.png"
              }
            ],
            "count": null,
            "description": "Auto playlist",
            "author": null
          },
          {
            "playlist_id": "VLPLCZQcydUIP07hMOwAXIag92l76d3z3Thv",
            "title": "Listen later",
            "thumbnails": [
              {
                "height": 192,
                "width": 192,
                "url": "https://yt3.ggpht.com/oGdMcu3X8XKqSc9QMRqV3rqznKuPScNylHcqmKiBfLE1TZ7gkqFJRwQX2rAiWyAOuLPM614fSDo=s192"
              },
              {
                "height": 576,
                "width": 576,
                "url": "https://yt3.ggpht.com/oGdMcu3X8XKqSc9QMRqV3rqznKuPScNylHcqmKiBfLE1TZ7gkqFJRwQX2rAiWyAOuLPM614fSDo=s576"
              }
            ],
            "count": null,
            "description": "Nick Dowsett • 20 tracks",
            "author": null
          },
          {
            "playlist_id": "VLRDCLAK5uy_lRzD6ZcGWU_ef3r4y7ifNYLiGmCCX_jIk",
            "title": "Deadly Hotlist",
            "thumbnails": [
              {
                "height": 226,
                "width": 226,
                "url": "https://lh3.googleusercontent.com/HJoX79I4ngSCHXjzEWHwWpvwlK2cMhbezyKN8I-lH06APDbjIAUymVCI1VmeB5EcrNwglLAB0Edlt1KL=w226-h226-l90-rj"
              },
              {
                "height": 544,
                "width": 544,
                "url": "https://lh3.googleusercontent.com/HJoX79I4ngSCHXjzEWHwWpvwlK2cMhbezyKN8I-lH06APDbjIAUymVCI1VmeB5EcrNwglLAB0Edlt1KL=w544-h544-l90-rj"
              }
            ],
            "count": null,
            "description": "YouTube Music • 50 songs",
            "author": null
          },
          {
            "playlist_id": "VLSE",
            "title": "Episodes for Later",
            "thumbnails": [
              {
                "height": 192,
                "width": 192,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/saved-episodes-@192.png"
              },
              {
                "height": 576,
                "width": 576,
                "url": "https://www.gstatic.com/youtube/media/ytm/images/pbg/saved-episodes-@576.png"
              }
            ],
            "count": null,
            "description": "Episodes you save for later",
            "author": null
          }
        ]);
        let expected: Vec<Playlist> = serde_json::from_value(expected).unwrap();
        assert_eq!(result, expected);
    }
    #[test]
    fn test_library_artists_dummy_json() {
        let testfile = std::fs::read_to_string("test_json/get_library_artists.json").unwrap();
        let testfile_json = serde_json::from_str(&testfile).unwrap();
        let json_crawler = JsonCrawler::from_json(testfile_json);
        let processed = ProcessedResult::from_raw(json_crawler, GetLibraryArtistsQuery::default());
        let result = processed.parse().unwrap();
        let expected = json!([{
          "channel_id" : "VLLM",
          "artist": "Liked Music",
          "byline": "test"
        }]);
        let expected: Vec<LibraryArtist> = serde_json::from_value(expected).unwrap();
        assert_eq!(result, expected);
    }
}

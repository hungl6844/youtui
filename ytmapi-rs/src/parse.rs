//! Results from parsing Innertube queries.
use crate::{
    common::{
        AlbumID, AlbumType, BrowseID, Explicit, PlaylistID, PodcastID, ProfileID, Thumbnail,
        VideoID, YoutubeID,
    },
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::*,
    process::{self, process_flex_column_item},
    query::Query,
    ChannelID,
};
use crate::{Error, Result};
pub use album::*;
pub use artist::*;
use const_format::concatcp;
pub use continuations::*;
pub use library::*;
pub use search::*;
use serde::{Deserialize, Serialize};

mod album;
mod artist;
mod continuations;
mod library;
mod search;

// TODO: Seal
// TODO: Implement for all types.
/// Trait to represent a YouTube struct that can be parsed.
pub trait Parse {
    type Output;
    fn parse(self) -> Result<Self::Output>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResults {
    pub top_results: Vec<TopResult>,
    pub artists: Vec<SearchResultArtist>,
    pub albums: Vec<SearchResultAlbum>,
    pub featured_playlists: Vec<SearchResultFeaturedPlaylist>,
    pub community_playlists: Vec<SearchResultCommunityPlaylist>,
    pub songs: Vec<SearchResultSong>,
    pub videos: Vec<SearchResultVideo>,
    pub podcasts: Vec<SearchResultPodcast>,
    pub episodes: Vec<SearchResultEpisode>,
    pub profiles: Vec<SearchResultProfile>,
}
#[derive(Debug, Clone)]
pub enum SearchResult {
    TopResult,
    Song(SearchResultSong),
    Album(SearchResultAlbum),
    Playlist(SearchResultPlaylist),
    Video,
    Artist(SearchResultArtist),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSongArtist {
    name: String,
    id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSongAlbum {
    pub name: Option<String>,
    id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopResult {
    result_type: SearchResultType,
    subscribers: Option<String>,
    thumbnails: Option<String>, //own type?
    // XXX: more to come
    artist_info: Option<ParsedSongList>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedSongList {
    artists: Vec<ParsedSongArtist>,
    album: Option<ParsedSongAlbum>,
    views: Option<String>,
    duration: Option<String>, // TODO: Duration as a time
    year: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// An artist search result.
pub struct SearchResultArtist {
    pub artist: String,
    pub subscribers: String,
    pub browse_id: Option<ChannelID<'static>>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A podcast search result.
pub struct SearchResultPodcast {
    pub title: String,
    pub publisher: String,
    pub podcast_id: Option<PodcastID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A podcast episode search result.
pub struct SearchResultEpisode {
    pub title: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A video search result.
pub struct SearchResultVideo {
    pub title: String,
    /// Note: Either Youtube channel name, or artist name.
    // Potentially can include link to channel.
    pub channel_name: String,
    pub views: String,
    pub length: String,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A profile search result.
pub struct SearchResultProfile {
    pub title: String,
    pub username: String,
    pub profile_id: ProfileID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// An album search result.
pub struct SearchResultAlbum {
    pub title: String,
    pub artist: String,
    pub year: String,
    pub explicit: Explicit,
    pub browse_id: Option<ChannelID<'static>>,
    pub album_type: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultSong {
    // Potentially can include links to artist and album.
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: String,
    pub plays: String,
    pub explicit: Explicit,
    pub video_id: Option<VideoID<'static>>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// A playlist search result may be a featured or community playlist.
pub enum SearchResultPlaylist {
    Featured(SearchResultFeaturedPlaylist),
    Community(SearchResultCommunityPlaylist),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A community playlist search result.
pub struct SearchResultCommunityPlaylist {
    pub title: String,
    pub author: String,
    pub views: String,
    pub playlist_id: Option<PlaylistID<'static>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A featured playlist search result.
pub struct SearchResultFeaturedPlaylist {
    pub title: String,
    pub author: String,
    pub songs: String,
    pub playlist_id: Option<PlaylistID<'static>>,
}

pub struct ProcessedResult<T>
where
    T: Query,
{
    query: T,
    json_crawler: JsonCrawler,
}
impl<T: Query> ProcessedResult<T> {
    pub fn from_raw(json_crawler: JsonCrawler, query: T) -> Self {
        Self {
            query,
            json_crawler,
        }
    }
    pub fn get_query(&self) -> &T {
        &self.query
    }
    pub fn get_crawler(&self) -> &JsonCrawler {
        &self.json_crawler
    }
}

// Should take FlexColumnItem? or Data?. Regular serde_json::Value could tryInto fixedcolumnitem also.
// Not sure if this should error.
// XXX: I think this should return none instead of error.
fn parse_song_artists(
    data: &mut JsonCrawlerBorrowed,
    col_idx: usize,
) -> Result<Vec<ParsedSongArtist>> {
    let mut artists = Vec::new();
    let Ok(flex_items) = process::process_flex_column_item(data, col_idx) else {
        return Ok(artists);
    };
    let Ok(flex_items_runs) = flex_items.navigate_pointer("/text/runs") else {
        return Ok(artists);
    };
    // https://github.com/sigma67/ytmusicapi/blob/master/ytmusicapi/parsers/songs.py
    // parse_song_artists_runs
    for mut i in flex_items_runs
        .into_array_iter_mut()
        .into_iter()
        .flatten()
        .step_by(2)
    {
        artists.push(ParsedSongArtist {
            name: i.take_value_pointer("/text")?,
            id: i.take_value_pointer(NAVIGATION_BROWSE_ID).ok(),
        });
    }
    Ok(artists)
}

fn parse_song_album(data: &mut JsonCrawlerBorrowed, col_idx: usize) -> Result<ParsedSongAlbum> {
    Ok(ParsedSongAlbum {
        name: parse_item_text(data, col_idx, 0).ok(),
        id: process_flex_column_item(data, col_idx)?
            .take_value_pointer(concatcp!("/text/runs/0", NAVIGATION_BROWSE_ID))
            .ok(),
    })
}

fn parse_item_text(
    item: &mut JsonCrawlerBorrowed,
    col_idx: usize,
    run_idx: usize,
) -> Result<String> {
    // Consider early return over the and_then calls.
    let pointer = format!("/text/runs/{run_idx}/text");
    process_flex_column_item(item, col_idx)?.take_value_pointer(pointer)
}

pub fn parse_top_result(mut data: JsonCrawlerBorrowed) -> Result<TopResult> {
    // Should be if-let?
    // XXX: The artist from the call to nav has quotation marks around it, causes error when
    // calleing get_search_result_type. I fix this with a hack.
    // TODO: i18n - search results can be in a different language.
    let st: String = data.take_value_pointer(SUBTITLE)?;
    let result_type = SearchResultType::try_from(&st)?;
    let _category = data.take_value_pointer(CARD_SHELF_TITLE)?;
    let thumbnails = data.take_value_pointer(THUMBNAILS).ok();
    let subscribers = if let SearchResultType::Artist = result_type {
        // TODO scrub / split subscribers.
        data.take_value_pointer(SUBTITLE2).ok()
    } else {
        todo!("Only handles Artist currently");
    };

    // TODO: artist_info
    let artist_info = Some(parse_song_runs(&data._take_json_pointer("/title/runs")?)?);
    Ok(TopResult {
        subscribers,
        result_type,
        thumbnails,
        artist_info,
    })
}

fn parse_song_runs(runs: &serde_json::Value) -> Result<ParsedSongList> {
    let mut artists = Vec::new();
    let year = None;
    let mut album = None;
    let views = None;
    let duration = None;
    if let serde_json::Value::Array(a) = runs {
        for (i, r) in a.iter().enumerate() {
            // Uneven items are always separators
            if (i % 2) == 1 {
                continue;
            }
            // TODO: Handle None
            let text = r.get("text").unwrap().to_string();
            // TODO: Handle None
            if let serde_json::Value::Object(_) = r.get("navigationEndpoint").unwrap() {
                // XXX: Is this definitely supposed to be an if let?
                let name = text;
                let id = r.pointer(NAVIGATION_BROWSE_ID).map(|id| id.to_string());
                // album
                // TODO: Cleanup unnecessary allocation
                if id
                    .clone()
                    .map_or(false, |item_id| item_id.contains("release_detail"))
                    || id
                        .clone()
                        .map_or(false, |item_id| item_id.starts_with("MPRE"))
                {
                    album = Some(ParsedSongAlbum {
                        name: Some(name),
                        id,
                    });
                } else {
                    //artist
                    artists.push(ParsedSongArtist { id, name });
                }
            } else {
                // XXX: Note, if artist doesn't have ID, will end up here and panic.
                todo!("Handle non artists or albums");
            }
        }
    } else {
        unreachable!("Assume input is valid");
    }
    Ok(ParsedSongList {
        artists,
        year,
        album,
        views,
        duration,
    })
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{process::JsonCloner, query::SearchQuery};

    use super::*;

    #[tokio::test]
    async fn test_all_processed_impl() {
        let query = SearchQuery::new("Beatles");
        let cloner = JsonCloner::from_string("{\"name\": \"John Doe\"}".to_string()).unwrap();
        let json_crawler = JsonCrawler::from_json_cloner(cloner);
        let json_crawler_clone = json_crawler.clone();
        let raw = ProcessedResult::from_raw(json_crawler, query.clone());
        assert_eq!(&query, raw.get_query());
        assert_eq!(&json_crawler_clone, raw.get_crawler());
    }
}

mod lyrics {
    use const_format::concatcp;

    use crate::common::browsing::Lyrics;
    use crate::nav_consts::{DESCRIPTION, DESCRIPTION_SHELF, RUN_TEXT, SECTION_LIST_ITEM};
    use crate::query::lyrics::GetLyricsQuery;
    use crate::Result;

    use super::ProcessedResult;

    impl<'a> ProcessedResult<GetLyricsQuery<'a>> {
        pub fn parse(self) -> Result<Lyrics> {
            let ProcessedResult { json_crawler, .. } = self;
            let mut description_shelf = json_crawler.navigate_pointer(concatcp!(
                "/contents",
                SECTION_LIST_ITEM,
                DESCRIPTION_SHELF
            ))?;
            Ok(Lyrics::new(
                description_shelf.take_value_pointer(DESCRIPTION)?,
                description_shelf.take_value_pointer(concatcp!("/footer", RUN_TEXT))?,
            ))
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::{
            common::{browsing::Lyrics, LyricsID},
            crawler::JsonCrawler,
            parse::ProcessedResult,
            process::JsonCloner,
            query::lyrics::GetLyricsQuery,
        };

        #[tokio::test]
        async fn test_get_lyrics_query() {
            // Intro - Notorious BIG - Ready To Die
            let path = std::path::Path::new("./test_json/get_lyrics_20231219.json");
            let file = tokio::fs::read_to_string(path)
                .await
                .expect("Expect file read to pass during tests");
            let json_clone = JsonCloner::from_string(file).unwrap();
            // Blank query has no bearing on function
            let query = GetLyricsQuery::new(LyricsID("".into()));
            let output =
                ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
                    .parse()
                    .unwrap();
            assert_eq!(
                output,
                Lyrics {
                    lyrics: "Push \r\nCome on, she almost there push, come on\r\nCome on, come on, push, it's almost there \r\nOne more time, come one\r\nCome on, push, baby, one more time \r\nHarder, harder, push it harder \r\nPush, push, come on \r\nOne more time, here it goes \r\nI see the head\r\nYeah, come on\r\nYeah, yeah\r\nYou did it, baby, yeah\r\n\r\nBut if you lose, don't ask no questions why\r\nThe only game you know is do or die\r\nAh-ha-ha\r\nHard to understand what a hell of a man\r\n\r\nHip hop the hippie the hippie\r\nTp the hip hop and you don't stop \r\nRock it out, baby bubba, to the boogie, the bang-bang\r\nThe boogie to the boogie that be\r\nNow what you hear is not a test, I'm rappin', to the beat \r\n\r\nGoddamn it, Voletta, what the fuck are you doin'?\r\nYou can't control that goddamn boy? (What?)\r\nI just saw Mr. Johnson, he told me he caught the motherfucking boy shoplifting \r\nWhat the fuck are you doing? (Kiss my black ass, motherfucker)\r\nYou can't control that god-, I don't know what the fuck to do with that boy\r\n(What the fuck do you want me to do?)\r\nIf if you can't fucking control that boy, I'ma send him\r\n(All you fucking do is bitch at me)\r\nBitch, bitch, I'ma send his motherfuckin' ass to a group home goddamnit, what?\r\nI'll smack the shit outta you bitch, what, what the fuck?\r\n(Kiss my black ass, motherfucker)\r\nYou're fuckin' up\r\n(Comin' in here smelling like sour socks you, dumb motherfucker) \r\n\r\nWhen I'm bustin' up a party I feel no guilt\r\nGizmo's cuttin' up for thee \r\nSuckers that's down with nei-\r\n\r\nWhat, nigga, you wanna rob them motherfuckin' trains, you crazy? \r\nYes, yes, motherfucker, motherfuckin' right, nigga, yes \r\nNigga, what the fuck, nigga? We gonna get-\r\nNigga, it's eighty-seven nigga, is you dead broke? \r\nYeah, nigga, but, but\r\nMotherfucker, is you broke, motherfucker? \r\nWe need to get some motherfuckin' paper, nigga \r\nNigga it's a train, ain't nobody never robbed no motherfuckin' train \r\nJust listen, man, is your mother givin' you money, nigga? \r\nMy moms don't give me shit nigga, it's time to get paid, nigga \r\nIs you with me? Motherfucker, is you with me? \r\nYeah, I'm with you, nigga, come on \r\nAlright then, nigga, lets make it happen then \r\nAll you motherfuckers get on the fuckin' floor \r\nGet on the motherfuckin' floor\r\nChill, give me all your motherfuckin' money \r\nAnd don't move, nigga\r\nI want the fuckin' jewelry \r\nGive me every fuckin' thing \r\nNigga, I'd shut the fuck up or I'ma blow your motherfuckin' brains out \r\nShut the fuck up, bitch, give me your fuckin' money, motherfucker\r\nFuck you, bitch, get up off that shit \r\nWhat the fuck you holdin' on to that shit for, bitch? \r\n\r\nI get money, money I got\r\nStunts call me honey if they feel real hot\r\n\r\nOpen C-74, Smalls \r\nMr. Smalls, let me walk you to the door \r\nSo how does it feel leavin' us? \r\nCome on, man, what kind of fuckin' question is that, man? \r\nTryin' to get the fuck up out this joint, dog \r\nYeah, yeah, you'll be back \r\nYou niggas always are \r\nGo ahead, man, what the fuck is you hollerin' about? \r\nYou won't see me up in this motherfucker no more \r\nWe'll see \r\nI got big plans nigga, big plans, hahaha".to_string(),
                    source: "Source: LyricFind".to_string()
                }
            );
        }
    }
}
mod watch {
    use const_format::concatcp;

    use crate::{
        common::watch::WatchPlaylist,
        crawler::JsonCrawlerBorrowed,
        nav_consts::{NAVIGATION_PLAYLIST_ID, TAB_CONTENT},
        query::watch::GetWatchPlaylistQuery,
        Result, VideoID,
    };

    use super::ProcessedResult;

    impl<'a> ProcessedResult<GetWatchPlaylistQuery<VideoID<'a>>> {
        // TODO: Continuations
        pub fn parse(self) -> Result<WatchPlaylist> {
            let ProcessedResult { json_crawler, .. } = self;
            let mut watch_next_renderer = json_crawler.navigate_pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer")?;
            let lyrics_id =
                get_tab_browse_id(&mut watch_next_renderer.borrow_mut(), 1)?.take_value()?;
            let mut results = watch_next_renderer.navigate_pointer(concatcp!(
                TAB_CONTENT,
                "/musicQueueRenderer/content/playlistPanelRenderer/contents"
            ))?;
            let playlist_id = results.as_array_iter_mut()?.find_map(|mut v| {
                v.take_value_pointer(concatcp!(
                    "/playlistPanelVideoRenderer",
                    NAVIGATION_PLAYLIST_ID
                ))
                .ok()
            });
            Ok(WatchPlaylist::new(playlist_id, lyrics_id))
        }
    }

    // Should be a Process function not Parse.
    fn get_tab_browse_id<'a>(
        watch_next_renderer: &'a mut JsonCrawlerBorrowed,
        tab_id: usize,
    ) -> Result<JsonCrawlerBorrowed<'a>> {
        // TODO: Safe option that returns none if tab doesn't exist.
        let path = format!("/tabs/{tab_id}/tabRenderer/endpoint/browseEndpoint/browseId");
        watch_next_renderer.borrow_pointer(path)
    }
}

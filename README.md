# About
Youtui - a simple TUI YouTube Music player written in Rust. Inspired by https://github.com/ccgauche/ytermusic/.

Ytmapi-rs - an asynchronous API for youtube music - using Google's internal API. Inspired by https://github.com/sigma67/ytmusicapi/.

This project is not supported by Google.
## Demo
Version as of 09/Nov/23 and is still a work in progress.
[![asciicast](https://asciinema.org/a/SOTRXdvkjM4vWHuwsWSDDDmBQ.svg)](https://asciinema.org/a/SOTRXdvkjM4vWHuwsWSDDDmBQ)
## How to install and run
- Clone the repository
- Build - note nightly rust required for async traits
- Give the application an authorisation header:
  1. Open YouTube Music in your browser - ensure you are logged in.
  1. Open web developer tools (F12).
  1. Open Network tab and locate a POST request to `music.youtube.com`.
  1. Copy the `Cookie` into a text file named `headers.txt` into your local youtui config directory (e.g ~/.config/youtui/ on Linux). Note you will need to create the directory if it does not exist.
Firefox example (Right click and Copy Value):
![image](https://github.com/nick42d/youtui/assets/133559267/c7fda32c-10bc-4ebe-b18e-ee17c13f6bd0)
Chrome example (Select manually and paste):
![image](https://github.com/nick42d/youtui/assets/133559267/bd2ec37b-1a78-490f-b313-694145bb4854)
### Linux dependencies note
- Youtui uses the Rodio library for playback which relies on Cpal https://github.com/rustaudio/cpal for ALSA support.
- The cpal readme mentions the that the ALSA development files are required which can be found in the following packages:
  - `libasound2-dev` (Debian / Ubuntu)
  - `alsa-lib-devel` (Fedora)
- The Reqwest library requires ssl - `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.
### Limitations
- The Rodio library used for playback does not currently support seeking or checking progress although there are PRs in progress for both. Progress updates are currently emulated with a ticker and may be slightly out, and seeking is not yet implemented.
## Coding constraints
App has been designed for me to learn Rust, and therefore I have implemented the following constraints to learn some features. I am aware these may not be the most efficient ways to code.
1. Avoid shared mutable state: 
The app will avoid shared mutable state primitives such as Mutex and RefCell and instead communicate via messaging.
1. Concurrency over parralelism: 
Where possible, the app will use use an asynchronous mode of operation (such as futures::join! and tokio::select) over parallel equivalents such as tokio::spawn and thread::spawn.
1. Avoid cloning: Where possible, the app will avoid cloning as a method to beat the borrow checker. Instead, we will try to safely borrow.
1. Encode state into the type system: Where possible use the type system to represent actions that are not possible in the current state.
## Design principles
I am aiming to follow the following design principles
1. Discoverability
The app should limit the cognitive load required to memorise commands and should instead provide mechanisms for the user to discover non-obvious commands. E.g commands that require multiple keypresses should display context menus for the subsequent presses like Kakoune or Helix.
## Roadmap
### Application
- [ ] Offline cache
- [ ] Proper configuration support
- [x] Implement improved download speed
- [ ] Streaming of buffered tracks
- [ ] Theming
### API
- [ ] Implement all endpoints
- [x] OAuth authentication
- [ ] i18n

|Endpoint | Implemented |
|--- | --- |
|GetArtist | [x] |
|GetAlbum | [x] |
|GetArtistAlbums | [x] |
|Search | [ ]\* |
|GetSearchSuggestions|[x]|
|GetHome|[ ]|
|GetAlbumBrowseId|[ ]|
|GetUser|[ ]|
|GetUserPlaylists|[ ]|
|GetSong|[ ]|
|GetSongRelated|[ ]|
|GetLyrics|[x]|
|GetTasteProfile|[ ]|
|SetTasteProfile|[ ]|
|GetMoodCategories|[ ]|
|GetMoodPlaylists|[ ]|
|GetCharts|[ ]|
|GetWatchPlaylist|[ ]\*|
|GetLibraryPlaylists|[ ]\*|
|GetLibrarySongs|[ ]|
|GetLibraryAlbums|[ ]|
|GetLibraryArtists|[ ]|
|GetLibrarySubscriptions|[ ]|
|GetLikedSongs|[ ]|
|GetHistory|[ ]|
|AddHistoryItem|[ ]|
|RemoveHistoryItem|[ ]|
|RateSong|[ ]|
|EditSongLibraryStatus|[ ]|
|RatePlaylist|[ ]|
|SubscribeArtists|[ ]|
|UnsubscribeArtists|[ ]|
|GetPlaylist|[ ]|
|CreatePlaylist|[ ]|
|EditPlaylist|[ ]|
|DeletePlaylist|[ ]|
|AddPlaylistItems|[ ]|
|RemovePlaylistItems|[ ]|
|GetLibraryUploadSongs|[ ]|
|GetLibraryUploadArtists|[ ]|
|GetLibraryUploadAlbums|[ ]|
|GetLibraryUploadArtist|[ ]|
|GetLibraryUploadAlbum|[ ]|
|UploadAlbum|[ ]|
|DeleteUploadEntity|[ ]|

\* search is partially implemented only 
- only returns artists
- does not implement continuations - only first x results returned.

\* get watch playlist is partially implemented only
- only returns playlist and lyrics ids
- does not implement continuations - only first x results returned.

\* get library playlist is partially implemented only
- does not implement continuations - only first x results returned.

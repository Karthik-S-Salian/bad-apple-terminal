# bad-apple-terminal
bad apple video is rendered on terminal by converting each frame to ascii art

### libraries
-------------------------------
| name     | usage       |
| ---------| ----------- |
|FFMPEG   :| frame extraction |
|crossterm:| rendering ascii frame in terminal|
|rodio    :| play audio|

[![wtach on youtube](https://img.youtube.com/vi/lTfW0bnWgkI/0.jpg)](https://www.youtube.com/watch?v=lTfW0bnWgkI)

[watch on youtube](https://www.youtube.com/watch?v=lTfW0bnWgkI)

## References
https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs

## Progress
- [x] minimal working project
- [x] use ffmpeg for downscaling image instead of manual code
- [x] compile with video and audio to single binary
- [x] try using ffmepg to extract audio instead of external audio file
- [ ] bug😭😭 audio wont play for first 2 sec [started after using ffmpeg to extract audio]
- [ ] audio and video synchronisation (no need looks clean on my machine😁)


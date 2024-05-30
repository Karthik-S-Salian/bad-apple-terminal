//terminal
use crossterm::cursor;
use crossterm::{style::Print, terminal, QueueableCommand};

use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::thread;

//audio
use rodio::Sink;

//video
extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

fn main() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let chars = [' ','-', '*', '#', '&', '@'];

    if let Ok(mut ictx) = input("data/video.mp4") {
        let mut stdout = io::stdout();

        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        let frame_rate = input.avg_frame_rate();
        let frame_duration = Duration::from_secs_f64(frame_rate.invert().into());
        let base_time = Instant::now();


        let mut terminal_size = get_terminal_size();
        let mut prvs_terminal_size:(u32,u32) = (0,0);
        let mut chars_vec:Vec<char> = Vec::new();

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::GRAY8,
            terminal_size.0,
            terminal_size.1,
            Flags::AREA,
        )?;

        let mut frame_index = 0;

        let mut receive_and_process_decoded_frames =
            |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                let mut decoded = Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let mut frame = Video::empty();


                    terminal_size = get_terminal_size();

                    if terminal_size!=prvs_terminal_size{
                        scaler = Context::get(
                            decoder.format(),
                            decoder.width(),
                            decoder.height(),
                            Pixel::GRAY8,
                            terminal_size.0,
                            terminal_size.1,
                            Flags::AREA,
                        )?;

                        prvs_terminal_size = terminal_size;

                        chars_vec = Vec::with_capacity((terminal_size.0*terminal_size.1) as usize);
                    }

                    scaler.run(&decoded, &mut frame)?;

                    let frame_data = frame.data(0);

                    for i in 0..terminal_size.1{
                        for j in 0..terminal_size.0{
                            let index = i as usize * frame.stride(0) + j as usize;
                            let c= chars[frame_data[index] as usize / (260 / (chars.len()))];
                            chars_vec.push(c);
                        }
                    }

                    stdout
                        .queue(terminal::Clear(terminal::ClearType::All))
                        .unwrap()
                        .queue(cursor::MoveTo(0, 0))
                        .unwrap()
                        .queue(Print(chars_vec.iter().collect::<String>()))
                        .unwrap()
                        .flush()
                        .unwrap();

                    chars_vec.clear();

                    frame_index += 1;

                    let expected_duration = frame_duration*frame_index;
                    match expected_duration.checked_sub(base_time.elapsed()) {
                        Some(diff)=>{
                            thread::sleep(diff);
                        },
                        None=>{}
                    } 
                }
                Ok(())
            };


        spawn_and_play_audio("data/audio.mp3".to_string());

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet)?;
                receive_and_process_decoded_frames(&mut decoder)?;
            }
            // break;
        }
        decoder.send_eof()?;
        receive_and_process_decoded_frames(&mut decoder)?;
    }

    Ok(())
}

fn spawn_and_play_audio(path:String){
    thread::spawn(|| {
        // Create a new sink
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        // Load  audio file 
        let file = std::fs::File::open(path).unwrap();
        let source = rodio::Decoder::new(std::io::BufReader::new(file)).unwrap();

        // Play the audio
        sink.append(source);
        sink.sleep_until_end();
    });
}

fn get_terminal_size() -> (u32, u32) {
    let (width, height) = terminal::size().unwrap();
    (width as u32, height as u32)
}

// use ffmpeg::format::context::Input;
// fn play_audio(ictx:Input)->Result<(),ffmpeg::Error>{
//     let input = ictx
//         .streams()
//         .best(Type::Audio)
//         .expect("could not find best audio stream");
//     let context = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
//     let mut decoder = context.decoder().audio()?;

//     decoder.set_parameters(input.parameters())?;

//     let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
//     let sink = Sink::try_new(&stream_handle).unwrap();

//     sink.append(decoder);
//     sink.play();
//     sink.sleep_until_end();

//     Ok(())
// }
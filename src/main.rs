// terminal
use crossterm::cursor;
use crossterm::{style::Print, terminal, QueueableCommand};

use rodio::Sink;
use std::env;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use ffmpeg::software::resampling::Context as Resampler;

// video
extern crate ffmpeg_next as ffmpeg;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

fn main() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let chars = [' ', '-', '*', '#', '&', '@'];

    // Include video.mp4 as bytes
    let video_bytes: &[u8] = include_bytes!("../data/video.mp4");
    let video_file_path = save_to_temp_file(video_bytes, "video.mp4");

    if let Ok(mut ictx) = input(&video_file_path) {
        let mut stdout = io::stdout();

        let input_video = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;

        let video_stream_index = input_video.index();

        decode_and_play_audio(video_file_path);

        let context_decoder =
            ffmpeg::codec::context::Context::from_parameters(input_video.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        let frame_rate = input_video.avg_frame_rate();
        let frame_duration = Duration::from_secs_f64(frame_rate.invert().into());
        let base_time = Instant::now();

        let mut terminal_size = get_terminal_size();
        let mut prvs_terminal_size: (u32, u32) = (0, 0);
        let mut chars_vec: Vec<char> = Vec::new();

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

                    if terminal_size != prvs_terminal_size {
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

                        chars_vec =
                            Vec::with_capacity((terminal_size.0 * terminal_size.1) as usize);
                    }

                    scaler.run(&decoded, &mut frame)?;

                    let frame_data = frame.data(0);

                    for i in 0..terminal_size.1 {
                        for j in 0..terminal_size.0 {
                            let index = i as usize * frame.stride(0) + j as usize;
                            let c = chars[frame_data[index] as usize / (260 / (chars.len()))];
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

                    let expected_duration = frame_duration * frame_index;
                    match expected_duration.checked_sub(base_time.elapsed()) {
                        Some(diff) => {
                            thread::sleep(diff);
                        }
                        None => {}
                    }
                }
                Ok(())
            };

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet)?;
                receive_and_process_decoded_frames(&mut decoder)?;
            }
        }
        decoder.send_eof()?;
        receive_and_process_decoded_frames(&mut decoder)?;
    }

    Ok(())
}

fn decode_and_play_audio(video_file_path: String) {
    thread::spawn(move || {
        ffmpeg::init().unwrap();

        let mut ictx = ffmpeg::format::input(&video_file_path).unwrap();

        let input = ictx
            .streams()
            .best(ffmpeg::media::Type::Audio)
            .expect("Audio stream not found");

        let audio_stream_index = input.index();

        let context_decoder =
            ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
        let mut decoder = context_decoder.decoder().audio().unwrap();

        // RESAMPLER: always resample to packed i16 (even if not needed) (bcs rodio excepts in that format)
        // for bad apple its in f32
        let mut resampler = Resampler::get(
            decoder.format(),
            decoder.channel_layout(),
            decoder.rate(),
            ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed),
            decoder.channel_layout(),
            decoder.rate(),
        )
        .unwrap();

        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let mut decoded = ffmpeg::frame::Audio::empty();
        let mut resampled = ffmpeg::frame::Audio::empty();

        for (stream, packet) in ictx.packets() {
            if stream.index() == audio_stream_index {
                if decoder.send_packet(&packet).is_ok() {
                    while decoder.receive_frame(&mut decoded).is_ok() {

                        // Resample the decoded frame
                        resampler.run(&decoded, &mut resampled).unwrap();

                        let samples: Vec<i16> = resampled
                            .data(0)
                            .chunks_exact(2)
                            .map(|b| i16::from_ne_bytes([b[0], b[1]]))
                            .collect();

                        let sample_rate = resampled.rate() as u32;
                        let channels = resampled.channels() as u16;

                        let source =
                            rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples);
                        sink.append(source);
                    }
                }
            }
        }

        // Flush decoder
        decoder.send_eof().ok();
        while decoder.receive_frame(&mut decoded).is_ok() {
            resampler.run(&decoded, &mut resampled).unwrap();
            let samples: Vec<i16> = resampled
                .data(0)
                .chunks_exact(2)
                .map(|b| i16::from_ne_bytes([b[0], b[1]]))
                .collect();
            let sample_rate = resampled.rate() as u32;
            let channels = resampled.channels() as u16;
            let source = rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples);
            sink.append(source);
        }

        sink.sleep_until_end();
    });
}

fn get_terminal_size() -> (u32, u32) {
    let (width, height) = terminal::size().unwrap();
    (width as u32, height as u32)
}

fn save_to_temp_file(data: &[u8], filename: &str) -> String {
    let temp_dir = env::temp_dir();
    let file_path = temp_dir.join(filename);
    std::fs::write(&file_path, data).expect("Failed to write to temp file");
    file_path.to_str().unwrap().to_string()
}

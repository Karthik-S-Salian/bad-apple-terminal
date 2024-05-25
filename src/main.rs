use crossterm::cursor;
use crossterm::{style::Print, terminal, QueueableCommand};
use std::io::{self, Write};
use std::time::{Duration, Instant};
use std::thread;
use rodio::Sink;
extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;

fn main() -> Result<(), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let chars = ['-', '*', '#', '&', '@'];

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

        println!("{:?}",frame_duration);

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::GRAY8,
            decoder.width(),
            decoder.height(),
            Flags::AREA,
        )?;

        let mut frame_index = 0;

        thread::spawn(|| {
            // Create a new sink
            let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
    
            // Load your audio file (replace "your_audio_file.wav" with the actual file path)
            let file = std::fs::File::open("data/audio.mp3").unwrap();
            let source = rodio::Decoder::new(std::io::BufReader::new(file)).unwrap();
    
            // Play the audio
            sink.append(source);
            sink.sleep_until_end();
        });

        let mut receive_and_process_decoded_frames =
            |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                let mut decoded = Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let mut frame = Video::empty();
                    scaler.run(&decoded, &mut frame)?;

                    // save_file(downsampled_image,terminal_size.0 as u32,terminal_size.1 as u32,frame_index).unwrap();

                    let terminal_size = terminal::size().unwrap();

                    let downsampled_image = area_downsample(
                        frame.data(0),
                        frame.width(),
                        frame.height(),
                        terminal_size.0 as u32,
                        terminal_size.1 as u32,
                    );

                    let chars_vec: String = downsampled_image
                        .iter()
                        .map(|&value| chars[value as usize / (260 / (chars.len()))])
                        .collect();

                    stdout
                        .queue(terminal::Clear(terminal::ClearType::All))
                        .unwrap()
                        .queue(cursor::MoveTo(0, 0))
                        .unwrap()
                        .queue(Print(chars_vec))
                        .unwrap()
                        .flush()
                        .unwrap();

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

// fn save_file(frame: Vec<u8>, width:u32,height:u32,index: usize) -> std::result::Result<(), std::io::Error> {
//   use std::fs::File;
// use std::io::prelude::*;
//     let mut file = File::create(format!("frame{}.ppm", index))?;
//     file.write_all(format!("P5\n{} {}\n128\n", width, height).as_bytes())?;
//     let byte_slice: &[u8] = &frame;
//     file.write_all(byte_slice)?;
//     Ok(())
// }

fn area_downsample(
    input: &[u8],
    in_width: u32,
    in_height: u32,
    out_width: u32,
    out_height: u32,
) -> Vec<u8> {
    // Check if output dimensions are valid
    if out_width > in_width || out_height > in_height {
        panic!("Output dimensions cannot be greater than input dimensions.");
    }

    // Calculate downsample ratios
    let width_ratio = in_width as f64 / out_width as f64;
    let height_ratio = in_height as f64 / out_height as f64;

    // Allocate memory for output image
    let mut output_img = Vec::with_capacity((out_width * out_height) as usize);
    unsafe {
        output_img.set_len((out_width * out_height) as usize);
    }

    for y in 0..out_height {
        for x in 0..out_width {
            // Calculate area covered by the output pixel in the input image
            let in_y_start = (y as f64 * height_ratio).floor() as u32;
            let in_y_end =
                f64::min((y as f64 + 1.0) * height_ratio, in_height as f64).floor() as u32;
            let in_x_start = (x as f64 * width_ratio).floor() as u32;
            let in_x_end = f64::min((x as f64 + 1.0) * width_ratio, in_width as f64).floor() as u32;

            // Accumulate intensity values within the area
            let mut intensity_sum: u32 = 0;
            for in_y in in_y_start..in_y_end {
                for in_x in in_x_start..in_x_end {
                    intensity_sum +=
                        input[in_y as usize * in_width as usize + in_x as usize] as u32;
                }
            }

            // Average intensity for the output pixel
            let area = (in_y_end - in_y_start) * (in_x_end - in_x_start);
            output_img[y as usize * out_width as usize + x as usize] =
                ((intensity_sum as u32 + area / 2) / area) as u8;
        }
    }

    output_img
}

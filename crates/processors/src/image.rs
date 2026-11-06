use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageFormat};
use std::io::Cursor;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageProcessorError {
    #[error("failed to decode image: {0}")]
    Decode(#[from] image::ImageError),

    #[error("image dimensions are zero or invalid")]
    InvalidDimensions,
}

#[derive(Debug, Clone)]
pub struct ThumbnailResult {
    pub original_width: u32,
    pub original_height: u32,
    pub thumb_width: u32,
    pub thumb_height: u32,
    pub png_bytes: Vec<u8>,
}

pub fn extract_animation_frames(
    bytes: &[u8],
    frames_dir: &Path,
    max_frames: usize,
) -> Option<usize> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).ok()?;
    let frames = decoder.into_frames();
    let mut count = 0;
    let _ = std::fs::create_dir_all(frames_dir);

    for frame in frames {
        if count >= max_frames {
            break;
        }
        if let Ok(f) = frame {
            let buffer = f.into_buffer();
            let frame_path = frames_dir.join(format!("frame_{}.png", count));
            if buffer.save_with_format(&frame_path, ImageFormat::Png).is_ok() {
                count += 1;
            }
        }
    }

    if count > 1 {
        Some(count)
    } else {
        None
    }
}

pub fn generate_thumbnail(
    image_bytes: &[u8],
    max_size: u32,
) -> Result<ThumbnailResult, ImageProcessorError> {
    let img = image::load_from_memory(image_bytes)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    if orig_w == 0 || orig_h == 0 {
        return Err(ImageProcessorError::InvalidDimensions);
    }

    let thumb = img.thumbnail(max_size, max_size);
    let (thumb_w, thumb_h) = (thumb.width(), thumb.height());

    let mut png_bytes = Vec::new();
    thumb.write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)?;

    Ok(ThumbnailResult {
        original_width: orig_w,
        original_height: orig_h,
        thumb_width: thumb_w,
        thumb_height: thumb_h,
        png_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_thumbnail_generation() {
        let mut img = RgbaImage::new(400, 200);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        let mut original_bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut original_bytes), ImageFormat::Png)
            .unwrap();

        let res = generate_thumbnail(&original_bytes, 100).unwrap();
        assert_eq!(res.original_width, 400);
        assert_eq!(res.original_height, 200);
        assert_eq!(res.thumb_width, 100);
        assert_eq!(res.thumb_height, 50);
        assert!(!res.png_bytes.is_empty());
    }
}

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::collections::HashMap;
use std::io::Cursor;

use super::model_resolver::ResolvedTexture;

/// Output size for rendered isometric block icons (square PNG).
const ICON_SIZE: u32 = 32;

/// Render an item texture to PNG bytes based on the resolved texture info.
///
/// - For Sprite items: returns the raw texture PNG bytes unchanged.
/// - For BlockIsometric items: renders an isometric composite of the three faces.
///
/// `all_textures` maps texture paths (e.g. "block/stone") to raw PNG bytes.
/// Texture references may include namespace prefixes (e.g. "minecraft:block/stone").
pub fn render_texture(
    resolved: &ResolvedTexture,
    all_textures: &HashMap<String, Vec<u8>>,
    namespace: &str,
) -> Option<Vec<u8>> {
    match resolved {
        ResolvedTexture::Sprite(tex_ref) => {
            let bytes = lookup_texture(tex_ref, all_textures, namespace)?;
            Some(bytes.clone())
        }
        ResolvedTexture::BlockIsometric { top, front, right } => {
            let top_bytes = lookup_texture(top, all_textures, namespace)?;
            let front_bytes = lookup_texture(front, all_textures, namespace)?;
            let right_bytes = lookup_texture(right, all_textures, namespace)?;

            let top_img = load_png(top_bytes)?;
            let front_img = load_png(front_bytes)?;
            let right_img = load_png(right_bytes)?;

            let composite = render_isometric(&top_img, &front_img, &right_img);
            encode_png(&composite)
        }
    }
}

/// Look up a texture by its reference string, handling namespace prefixes.
fn lookup_texture<'a>(
    tex_ref: &str,
    all_textures: &'a HashMap<String, Vec<u8>>,
    default_namespace: &str,
) -> Option<&'a Vec<u8>> {
    // Strip namespace prefix if present (e.g. "minecraft:block/stone" -> "block/stone")
    let path = if let Some(idx) = tex_ref.find(':') {
        &tex_ref[idx + 1..]
    } else {
        tex_ref
    };

    // Try direct lookup
    if let Some(bytes) = all_textures.get(path) {
        return Some(bytes);
    }

    // Try with namespace prefix
    let with_ns = format!("{}:{}", default_namespace, path);
    if let Some(bytes) = all_textures.get(&with_ns) {
        return Some(bytes);
    }

    None
}

/// Load raw PNG bytes into a DynamicImage
fn load_png(bytes: &[u8]) -> Option<DynamicImage> {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()
}

/// Encode an RgbaImage to PNG bytes
fn encode_png(img: &RgbaImage) -> Option<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
    Some(buf.into_inner())
}

/// Render an isometric block composite from top, front (left), and right face textures.
///
/// Produces a classic Minecraft inventory-style isometric block:
///
/// ```text
///        /\
///       /  \       <- top face (diamond)
///      /    \
///     /------\
///     |\    /|
///     | \  / |     <- left face (front)  |  right face
///     |  \/  |
///     \------/
/// ```
///
/// The 32x32 output maps a 16x16 block into an isometric projection.
/// Each source texel maps to approximately 2x1 screen pixels.
fn render_isometric(
    top_tex: &DynamicImage,
    front_tex: &DynamicImage,
    right_tex: &DynamicImage,
) -> RgbaImage {
    let mut canvas = ImageBuffer::from_pixel(ICON_SIZE, ICON_SIZE, Rgba([0, 0, 0, 0]));

    let top = top_tex.resize_exact(16, 16, image::imageops::FilterType::Nearest);
    let front = front_tex.resize_exact(16, 16, image::imageops::FilterType::Nearest);
    let right = right_tex.resize_exact(16, 16, image::imageops::FilterType::Nearest);

    // The isometric block is drawn in a 32x32 canvas.
    // Think of the block as a cube seen from above-right.
    //
    // Coordinate system: the block occupies 16x16x16 source pixels.
    // The isometric projection maps (bx, by, bz) to screen:
    //   sx = origin_x + bx - bz
    //   sy = origin_y + (bx + bz) / 2 - by
    //
    // We draw faces from back to front (painter's algorithm):
    //   1. Top face (y=16, varies x and z)
    //   2. Left/front face (z=16, varies x and y)
    //   3. Right face (x=16, varies z and y)
    //
    // Origin is chosen so the block is centered in the 32x32 canvas.
    // Top-center of the diamond is at (16, 0) when bx=0, bz=0, by=16.
    // We use origin (16, 16) so:
    //   Top vertex (bx=0, bz=0, by=16): sx=16, sy=0
    //   Bottom vertex (bx=16, bz=16, by=0): sx=16, sy=32

    let origin_x: f32 = 16.0;
    let origin_y: f32 = 16.0;

    // Draw top face (y = block height = 16 in block coords)
    // Texture u maps to block x, texture v maps to block z
    for tv in 0..16u32 {
        for tu in 0..16u32 {
            let pixel = top.get_pixel(tu, tv);
            if pixel[3] == 0 {
                continue;
            }
            let bx = tu as f32;
            let bz = tv as f32;
            let by: f32 = 16.0;

            let sx = origin_x + bx - bz;
            let sy = origin_y + (bx + bz) / 2.0 - by;

            let pixel = shade_pixel(pixel, 1.0);
            put_iso_pixel(&mut canvas, sx, sy, pixel);
        }
    }

    // Draw left (front) face (z = 16 in block coords, facing viewer-left)
    // Texture u maps to block x (0..16), texture v maps to block y (16..0, top to bottom)
    for tv in 0..16u32 {
        for tu in 0..16u32 {
            let pixel = front.get_pixel(tu, tv);
            if pixel[3] == 0 {
                continue;
            }
            let bx = tu as f32;
            let bz: f32 = 16.0;
            let by = 16.0 - tv as f32; // v=0 is top of face

            let sx = origin_x + bx - bz;
            let sy = origin_y + (bx + bz) / 2.0 - by;

            let pixel = shade_pixel(pixel, 0.8);
            put_iso_pixel(&mut canvas, sx, sy, pixel);
        }
    }

    // Draw right face (x = 16 in block coords, facing viewer-right)
    // Texture u maps to block z (16..0 for correct orientation), texture v maps to block y (16..0)
    for tv in 0..16u32 {
        for tu in 0..16u32 {
            let pixel = right.get_pixel(tu, tv);
            if pixel[3] == 0 {
                continue;
            }
            let bx: f32 = 16.0;
            let bz = 16.0 - tu as f32; // u=0 is the left edge of the face (high z)
            let by = 16.0 - tv as f32;

            let sx = origin_x + bx - bz;
            let sy = origin_y + (bx + bz) / 2.0 - by;

            let pixel = shade_pixel(pixel, 0.6);
            put_iso_pixel(&mut canvas, sx, sy, pixel);
        }
    }

    canvas
}

/// Place a pixel at a floating-point screen position.
/// Since isometric projection produces half-pixel offsets, we round to nearest.
fn put_iso_pixel(canvas: &mut RgbaImage, sx: f32, sy: f32, pixel: Rgba<u8>) {
    let x = sx.round() as i32;
    let y = sy.round() as i32;
    if x >= 0 && x < ICON_SIZE as i32 && y >= 0 && y < ICON_SIZE as i32 {
        blend_pixel(canvas, x as u32, y as u32, pixel);
    }
}

/// Apply a brightness multiplier to a pixel (for face shading).
fn shade_pixel(pixel: Rgba<u8>, factor: f32) -> Rgba<u8> {
    Rgba([
        (pixel[0] as f32 * factor).min(255.0) as u8,
        (pixel[1] as f32 * factor).min(255.0) as u8,
        (pixel[2] as f32 * factor).min(255.0) as u8,
        pixel[3],
    ])
}

/// Alpha-blend a source pixel onto a canvas pixel.
fn blend_pixel(canvas: &mut RgbaImage, x: u32, y: u32, src: Rgba<u8>) {
    if x >= canvas.width() || y >= canvas.height() {
        return;
    }
    let dst = canvas.get_pixel(x, y);
    let sa = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);

    if out_a == 0.0 {
        return;
    }

    let blend = |s: u8, d: u8| -> u8 {
        ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a).min(255.0) as u8
    };

    canvas.put_pixel(
        x,
        y,
        Rgba([
            blend(src[0], dst[0]),
            blend(src[1], dst[1]),
            blend(src[2], dst[2]),
            (out_a * 255.0) as u8,
        ]),
    );
}

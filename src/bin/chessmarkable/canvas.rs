pub use libremarkable::framebuffer::{
    cgmath::Point2, cgmath::Vector2, common::color, common::mxcfb_rect, common::DISPLAYHEIGHT,
    common::DISPLAYWIDTH, core::Framebuffer, storage::rgbimage_from_u8_slice, FramebufferBase,
    FramebufferDraw, FramebufferIO, FramebufferRefresh,
};
use libremarkable::framebuffer::{
    common::display_temp, common::dither_mode, common::waveform_mode, refresh::PartialRefreshMode,
};
use libremarkable::image;
use std::ops::DerefMut;
use libremarkable::cgmath::vec2;

pub struct Canvas<'a> {
    framebuffer: Box<Framebuffer<'a>>,
}

impl<'a> Canvas<'a> {
    pub fn new() -> Self {
        Self {
            framebuffer: Box::new(Framebuffer::from_path("/dev/fb0")),
        }
    }

    pub fn framebuffer_mut(&mut self) -> &'static mut Framebuffer<'static> {
        unsafe {
            std::mem::transmute::<_, &'static mut Framebuffer<'static>>(
                self.framebuffer.deref_mut(),
            )
        }
    }

    pub fn clear(&mut self) {
        self.framebuffer_mut().clear();
    }

    pub fn update_full(&mut self) -> u32 {
        self.framebuffer_mut().full_refresh(
            waveform_mode::WAVEFORM_MODE_GC16,
            display_temp::TEMP_USE_REMARKABLE_DRAW,
            dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
            0,
            true,
        )
    }

    pub fn update_partial(&mut self, region: &mxcfb_rect) -> u32 {
        self.framebuffer_mut().partial_refresh(
            region,
            PartialRefreshMode::Async,
            waveform_mode::WAVEFORM_MODE_GC16_FAST,
            display_temp::TEMP_USE_REMARKABLE_DRAW,
            dither_mode::EPDC_FLAG_USE_REMARKABLE_DITHER,
            0, // See documentation on DRAWING_QUANT_BITS in libremarkable/framebuffer/common.rs
            false,
        )
    }

    pub fn wait_for_update(&mut self, update_marker: u32) {
        self.framebuffer_mut().wait_refresh_complete(update_marker);
    }

    //Long text with draw_text layers on top of each other ending up in garbled output
    //This is a quick hack
    //I've found that the remarkable can do about a 100 characters at 35.0 font size
    //if you're looking for a default that fits the whole screen
    pub fn draw_multi_line_text(&mut self, x_pos: Option<i32>, y_pos: i32, text: &str, chars_per_line: usize, size: f32) -> mxcfb_rect {
        if text.len() > 0 {
            let mut vec_of_text = Vec::new();
            let mut peekable = text.chars().peekable();
            let mut last_text_height = 0;
            let mut last_text_y = y_pos;
            while peekable.peek().is_some() {
                let chunk: String = peekable.by_ref().take(chars_per_line).collect();
                let text_rect = self.draw_text(Point2{ x: x_pos, y: Some(last_text_y + last_text_height) }, &*chunk, size);
                last_text_height = text_rect.height as i32 + 20;
                last_text_y = text_rect.top as i32;
                vec_of_text.push(text_rect);
            }
            mxcfb_rect {
                top: vec_of_text.first().unwrap().top,
                left: vec_of_text.iter().map(|&rec| rec.left).min().unwrap(),
                width: vec_of_text.iter().map(|&rec| rec.width).max().unwrap(),
                height: vec_of_text.iter().map(|&rec| rec.height).sum()
            }
        } else {
            mxcfb_rect {
                top: 0,
                left: 0,
                width: 0,
                height: 0
            }
        }
    }

    pub fn draw_text(&mut self, pos: Point2<Option<i32>>, text: &str, size: f32) -> mxcfb_rect {
        let mut pos = pos;
        if pos.x.is_none() || pos.y.is_none() {
            // Do dryrun to get text size
            let rect = self.framebuffer_mut().draw_text(
                Point2 {
                    x: 0.0,
                    y: DISPLAYHEIGHT as f32,
                },
                text.to_owned(),
                size,
                color::BLACK,
                true,
            );

            if pos.x.is_none() {
                // Center horizontally
                pos.x = Some(DISPLAYWIDTH as i32 / 2 - rect.width as i32 / 2);
            }

            if pos.y.is_none() {
                // Center vertically
                pos.y = Some(DISPLAYHEIGHT as i32 / 2 - rect.height as i32 / 2);
            }
        }
        let pos = Point2 {
            x: pos.x.unwrap() as f32,
            y: pos.y.unwrap() as f32,
        };

        self.framebuffer_mut()
            .draw_text(pos, text.to_owned(), size, color::BLACK, false)
    }

    fn draw_box(&mut self, pos: Point2<i32>, size: Vector2<u32>, border_px: u32, c: color) -> mxcfb_rect  {
        let top_left = pos;
        let top_right = pos + vec2(size.x as i32, 0);
        let bottom_left = pos + vec2(0, size.y as i32);
        let bottom_right = bottom_left + vec2(size.x as i32, 0);

        // top horizontal
        self.framebuffer_mut()
            .draw_line(top_left, top_right, border_px, c);

        self.framebuffer_mut()
            .draw_line(bottom_left, bottom_right, border_px, c);
        mxcfb_rect {
            top: pos.y as u32,
            left: pos.x as u32,
            width: size.x,
            height: size.y,
        }
    }

    pub fn draw_rect(
        &mut self,
        pos: Point2<Option<i32>>,
        size: Vector2<u32>,
        border_px: u32,
    ) -> mxcfb_rect {
        let mut pos = pos;
        if pos.x.is_none() || pos.y.is_none() {
            if pos.x.is_none() {
                // Center horizontally
                pos.x = Some(DISPLAYWIDTH as i32 / 2 - size.x as i32 / 2);
            }

            if pos.y.is_none() {
                // Center vertically
                pos.y = Some(DISPLAYHEIGHT as i32 / 2 - size.y as i32 / 2);
            }
        }
        let pos = Point2 {
            x: pos.x.unwrap(),
            y: pos.y.unwrap(),
        };

        self.framebuffer_mut()
            .draw_rect(pos, size, border_px, color::BLACK);
        mxcfb_rect {
            top: pos.y as u32,
            left: pos.x as u32,
            width: size.x,
            height: size.y,
        }
    }

    pub fn fill_rect(
        &mut self,
        pos: Point2<Option<i32>>,
        size: Vector2<u32>,
        clr: color,
    ) -> mxcfb_rect {
        let mut pos = pos;
        if pos.x.is_none() || pos.y.is_none() {
            if pos.x.is_none() {
                // Center horizontally
                pos.x = Some(DISPLAYWIDTH as i32 / 2 - size.x as i32 / 2);
            }

            if pos.y.is_none() {
                // Center vertically
                pos.y = Some(DISPLAYHEIGHT as i32 / 2 - size.y as i32 / 2);
            }
        }
        let pos = Point2 {
            x: pos.x.unwrap(),
            y: pos.y.unwrap(),
        };

        self.framebuffer_mut().fill_rect(pos, size, clr);
        mxcfb_rect {
            top: pos.y as u32,
            left: pos.x as u32,
            width: size.x,
            height: size.y,
        }
    }

    pub fn draw_button(
        &mut self,
        pos: Point2<Option<i32>>,
        text: &str,
        font_size: f32,
        vgap: u32,
        hgap: u32,
    ) -> mxcfb_rect {
        let text_rect = self.draw_text(pos, text, font_size);
        self.draw_rect(
            Point2 {
                x: Some((text_rect.left - hgap) as i32),
                y: Some((text_rect.top - vgap) as i32),
            },
            Vector2 {
                x: hgap + text_rect.width + hgap,
                y: vgap + text_rect.height + vgap,
            },
            5,
        )
    }

    //Text size seems to vary
    //This ignores text size so that boxes line up deterministically
    //Text ends up a bit off center though unfortunately
    pub fn draw_box_button(
        &mut self,
        y_pos: i32,
        y_height: u32,
        text: &str,
        font_size: f32,
    ) -> mxcfb_rect {
        let button_hitbox = self.draw_box(
            Point2 {
                x: 0,
                y: y_pos,
            },
            Vector2 {
                x: DISPLAYWIDTH as u32,
                y: y_height,
            },
            5,
            color::BLACK
        );
        self.draw_text(Point2 { x: None, y: Some((button_hitbox.top + y_height / 2) as i32) }, text, font_size);
        button_hitbox
    }

    /// Image that can be overlayed white respecting the previous pixels.
    /// This way transparent images can work.
    fn calc_overlay_image(
        &mut self,
        pos: Point2<i32>,
        img: &image::DynamicImage,
    ) -> image::RgbImage {
        let rgba = img.to_rgba();
        let mut rgb = img.to_rgb();

        let orig_rgb888 = rgbimage_from_u8_slice(
            rgba.width(),
            rgba.height(),
            &self
                .framebuffer_mut()
                .dump_region(mxcfb_rect {
                    top: pos.y as u32,
                    left: pos.x as u32,
                    width: rgba.width(),
                    height: rgba.height(),
                })
                .unwrap(),
        )
        .unwrap();

        for (x, y, pixel) in rgba.enumerate_pixels() {
            let color_pix = [
                pixel[0] as f32 / 255.0,
                pixel[1] as f32 / 255.0,
                pixel[2] as f32 / 255.0,
            ];
            let color_alpha = (255 - pixel[3]) as f32 / 255.0;
            let orig_pixel = orig_rgb888.get_pixel(x, y);
            let new_rgb_f32 = image::Rgb([
                color_pix[0] * (1.0 - color_alpha) + (orig_pixel[0] as f32 / 255.0) * color_alpha,
                color_pix[1] * (1.0 - color_alpha) + (orig_pixel[1] as f32 / 255.0) * color_alpha,
                color_pix[2] * (1.0 - color_alpha) + (orig_pixel[2] as f32 / 255.0) * color_alpha,
            ]);

            let new_rgb_u8: image::Rgb<u8> = image::Rgb([
                (new_rgb_f32[0] * 255.0) as u8,
                (new_rgb_f32[1] * 255.0) as u8,
                (new_rgb_f32[2] * 255.0) as u8,
            ]);

            rgb.put_pixel(x, y, new_rgb_u8);
        }

        rgb
    }

    pub fn draw_image(
        &mut self,
        pos: Point2<i32>,
        img: &image::DynamicImage,
        is_transparent: bool,
    ) -> mxcfb_rect {
        let rgb_img = if is_transparent {
            self.calc_overlay_image(pos, img)
        } else {
            img.to_rgb()
        };

        self.framebuffer_mut().draw_image(&rgb_img, pos);
        mxcfb_rect {
            top: pos.y as u32,
            left: pos.x as u32,
            width: rgb_img.width(),
            height: rgb_img.height(),
        }
    }

    pub fn is_hitting(pos: Point2<u16>, hitbox: mxcfb_rect) -> bool {
        (pos.x as u32) >= hitbox.left
            && (pos.x as u32) < (hitbox.left + hitbox.width)
            && (pos.y as u32) >= hitbox.top
            && (pos.y as u32) < (hitbox.top + hitbox.height)
    }
}

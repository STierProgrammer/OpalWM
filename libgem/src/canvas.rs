pub use libopal::window::Pixel;
use libopal::window::Window;

pub trait DrawingCanvas {
    fn draw_pixel(&mut self, x: u32, y: u32, pixel: Pixel);

    fn width(&self) -> u32;
    fn height(&self) -> u32;

    #[inline]
    fn draw_rect_points(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, pixel: Pixel) {
        // includes x2 and y2
        let width = (x2 - x1) + 1;
        let height = (y2 - y1) + 1;

        self.draw_rect(x1, y1, width, height, pixel);
    }

    #[inline]
    fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, pixel: Pixel) {
        for row in 0..height {
            for col in 0..width {
                self.draw_pixel(col + x, row + y, pixel);
            }
        }
    }

    #[inline]
    fn draw_line(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, pixel: Pixel) {
        self.draw_rect_points(x0, y0, x1, y1, pixel)
    }

    /// Draw a circle, starting at (x, y) which is the top-left corner of the circle,
    /// and ending at (x + radius*2, y + radius*2).
    #[inline]
    fn draw_circle(&mut self, x: u32, y: u32, radius: u32, border_color: Pixel, fill_color: Pixel) {
        let x = x + (radius * 2);
        let y = y + (radius * 2);

        let mut f = 1 - radius as i32;
        let mut ddf_x = 1;
        let mut ddf_y = -2 * radius as i32;

        let mut xx = 0u32;
        let mut yy = radius;

        while xx < yy {
            if f >= 0 {
                yy -= 1;
                ddf_y += 2;
                f += ddf_y;
            }

            xx += 1;
            ddf_x += 2;
            f += ddf_x;

            // Bottom Right corner
            self.draw_pixel(x + xx - radius, y + yy - radius, border_color);
            self.draw_pixel(x + yy - radius, y + xx - radius, border_color);
            // Top Right corner
            self.draw_pixel(x + xx - radius, y - yy - radius, border_color);
            self.draw_pixel(x + yy - radius, y - xx - radius, border_color);
            // Bottom Left corner
            self.draw_pixel(x - xx - radius, y + yy - radius, border_color);
            self.draw_pixel(x - yy - radius, y + xx - radius, border_color);
            // Top Left corner
            self.draw_pixel(x - xx - radius, y - yy - radius, border_color);
            self.draw_pixel(x - yy - radius, y - xx - radius, border_color);
        }
    }

    /// Draws a rounded rectangle on the canvas with the given border color and fills with the given fill_color
    fn draw_round_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
        border_color: Pixel,
        fill_color: Pixel,
    ) {
        // Draws two corners of a rounded rectangle, and then connects them with a line of fill_color
        let mut draw_2corners = |x0: u32, x1: u32, y: u32, top: bool| {
            let x0 = x0 + (radius * 2);
            let x1 = x1 + (radius * 2);

            let y = y + (radius * 2);

            let mut f = 1 - radius as i32;
            let mut ddf_x = 1;
            let mut ddf_y = -2 * radius as i32;

            let mut xx = 0u32;
            let mut yy = radius;

            while xx < yy {
                let last_yy = yy;
                let last_xx = xx;

                if f >= 0 {
                    yy -= 1;
                    ddf_y += 2;
                    f += ddf_y;
                }

                xx += 1;
                ddf_x += 2;
                f += ddf_x;

                match top {
                    // TODO: shit & broken make better
                    false => {
                        // Bottom Left corner
                        self.draw_pixel(x0 - xx - radius, y + yy - radius, border_color);
                        self.draw_pixel(x0 - yy - radius, y + xx - radius, border_color);

                        // Bottom Right corner
                        self.draw_pixel(x1 + xx - radius, y + yy - radius, border_color);
                        self.draw_pixel(x1 + yy - radius, y + xx - radius, border_color);

                        if yy != last_yy {
                            // Connect from x0 to x1
                            let line_x0 = x0 - xx - radius + 1;
                            let line_y0 = y + yy - radius;
                            let line_x1 = x1 + xx - radius - 1;

                            self.draw_line(line_x0, line_y0, line_x1, line_y0, fill_color);
                        }

                        if xx != last_xx {
                            // Connect from x0 to x1 (rotated)
                            let line_x0 = x0 - yy - radius + 1;
                            let line_y0 = y + xx - radius;
                            let line_x1 = x1 + yy - radius - 1;

                            self.draw_line(line_x0, line_y0, line_x1, line_y0, fill_color);
                        }
                    }
                    true => {
                        // Top Left corner
                        self.draw_pixel(x0 - xx - radius, y - yy - radius, border_color);
                        self.draw_pixel(x0 - yy - radius, y - xx - radius, border_color);

                        // Top Right corner
                        self.draw_pixel(x1 + xx - radius, y - yy - radius, border_color);
                        self.draw_pixel(x1 + yy - radius, y - xx - radius, border_color);

                        // only if yy changed
                        if yy != last_yy {
                            // Connect from x0 to x1
                            let line_x0 = x0 - xx - radius + 1;
                            let line_y0 = y - yy - radius;
                            let line_x1 = x1 + xx - radius - 1;

                            self.draw_line(line_x0, line_y0, line_x1, line_y0, fill_color);
                        }
                        if xx != last_xx {
                            // Connect from x0 to x1 (rotated)
                            let line_x0 = x0 - yy - radius + 1;
                            let line_y0 = y - xx - radius;
                            let line_x1 = x1 + yy - radius - 1;

                            self.draw_line(line_x0, line_y0, line_x1, line_y0, fill_color);
                        }
                    }
                }
            }
        };
        let x0 = x;
        let y0 = y;
        let x1 = (x + width) - 1;
        let y1 = (y + height) - 1;

        draw_2corners(x0, x1 - (radius * 2), y0, true);
        draw_2corners(x0, x1 - (radius * 2), y1 - (radius * 2), false);

        // Draws the border
        // Top line
        self.draw_line(x0 + radius, y0, x1 - radius, y0, border_color);
        // Bottom line
        self.draw_line(x0 + radius, y1, x1 - radius, y1, border_color);
        // Left line
        self.draw_line(x0, y0 + radius, x0, y1 - radius, border_color);
        // Right line
        self.draw_line(x1, y0 + radius, x1, y1 - radius, border_color);

        for y in (y0 + radius)..=(y1 - radius) {
            self.draw_line(x0 + 1, y, x1 - 1, y, fill_color);
        }
    }
}

impl DrawingCanvas for Window {
    #[inline]
    fn height(&self) -> u32 {
        self.height()
    }

    #[inline]
    fn width(&self) -> u32 {
        self.width()
    }

    #[inline]
    fn draw_pixel(&mut self, x: u32, y: u32, pixel: Pixel) {
        let index = (y * self.width() + x) as usize;
        self.pixels_mut()[index] = pixel;
    }
}

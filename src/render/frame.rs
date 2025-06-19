/// Represents a frame of the NES screen
/// Supports advanced effects like split scroll and palette changes
pub struct Frame {
    pub data: Vec<u8>,

    // Buffers for advanced effects
    pub background_buffer: Vec<u8>,
    pub sprite_buffer: Vec<u8>,
    pub priority_buffer: Vec<bool>, // true = sprite has priority
}

impl Frame {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 240;

    pub fn new() -> Self {
        let buffer_size = Frame::WIDTH * Frame::HEIGHT * 3;
        Frame {
            data: vec![0; buffer_size],
            background_buffer: vec![0; buffer_size],
            sprite_buffer: vec![0; buffer_size],
            priority_buffer: vec![false; Frame::WIDTH * Frame::HEIGHT],
        }
    }

    /// Sets a pixel with improved bounds checking
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        if x < Frame::WIDTH && y < Frame::HEIGHT {
            let base = y * 3 * Frame::WIDTH + x * 3;
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }

    /// Sets a background pixel in the separate buffer
    pub fn set_background_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        if x < Frame::WIDTH && y < Frame::HEIGHT {
            let base = y * 3 * Frame::WIDTH + x * 3;
            self.background_buffer[base] = rgb.0;
            self.background_buffer[base + 1] = rgb.1;
            self.background_buffer[base + 2] = rgb.2;
        }
    }

    /// Sets a sprite pixel in the separate buffer
    pub fn set_sprite_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8), priority: bool) {
        if x < Frame::WIDTH && y < Frame::HEIGHT {
            let base = y * 3 * Frame::WIDTH + x * 3;
            let pixel_index = y * Frame::WIDTH + x;

            self.sprite_buffer[base] = rgb.0;
            self.sprite_buffer[base + 1] = rgb.1;
            self.sprite_buffer[base + 2] = rgb.2;
            self.priority_buffer[pixel_index] = priority;
        }
    }

    /// Combines the background and sprite buffers according to priorities
    pub fn composite_buffers(&mut self) {
        for y in 0..Frame::HEIGHT {
            for x in 0..Frame::WIDTH {
                let pixel_index = y * Frame::WIDTH + x;
                let base = y * 3 * Frame::WIDTH + x * 3;

                // Start with the background
                self.data[base] = self.background_buffer[base];
                self.data[base + 1] = self.background_buffer[base + 1];
                self.data[base + 2] = self.background_buffer[base + 2];

                // Check if there is a non-transparent sprite at this position
                let sprite_transparent = self.sprite_buffer[base] == 0
                    && self.sprite_buffer[base + 1] == 0
                    && self.sprite_buffer[base + 2] == 0;

                // If the sprite is not transparent, apply it according to its priority
                if !sprite_transparent {
                    let _sprite_behind_bg = !self.priority_buffer[pixel_index];
                    let bg_transparent = self.background_buffer[base] == 0
                        && self.background_buffer[base + 1] == 0
                        && self.background_buffer[base + 2] == 0;

                    // Sprite is visible if:
                    // - It is in front of the background (priority = true), OR
                    // - It is behind but the background is transparent
                    if self.priority_buffer[pixel_index] || bg_transparent {
                        self.data[base] = self.sprite_buffer[base];
                        self.data[base + 1] = self.sprite_buffer[base + 1];
                        self.data[base + 2] = self.sprite_buffer[base + 2];
                    }
                }
            }
        }
    }

    /// Clears all buffers
    pub fn clear(&mut self) {
        self.data.fill(0);
        self.background_buffer.fill(0);
        self.sprite_buffer.fill(0);
        self.priority_buffer.fill(false);
    }

    /// Returns the frame dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (Frame::WIDTH, Frame::HEIGHT)
    }
}

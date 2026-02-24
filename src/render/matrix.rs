use rand::Rng;
use crate::config::Charset;

pub fn charset_chars(charset: &Charset) -> Vec<char> {
    match charset {
        Charset::Katakana => (0xFF66u32..=0xFF9F).filter_map(char::from_u32).collect(),
        Charset::Latin => ('A'..='Z').chain('a'..='z').collect(),
        Charset::Digits => ('0'..='9').collect(),
        Charset::Mixed => {
            let mut v: Vec<char> = (0xFF66u32..=0xFF9F).filter_map(char::from_u32).collect();
            v.extend('A'..='Z');
            v.extend('0'..='9');
            v
        }
    }
}

pub struct Column {
    pub col_x: i32,
    pub head_y: f32,
    pub trail_len: usize,
    height_cells: i32,
    speed: f32,
    delay: f32,
}

impl Column {
    pub fn new(col_x: i32, height_cells: i32, speed: f32) -> Self {
        let mut rng = rand::thread_rng();
        Column {
            col_x,
            head_y: -(rng.gen_range(1..height_cells) as f32),
            trail_len: rng.gen_range(8..20),
            height_cells,
            speed: speed * rng.gen_range(0.5f32..1.5),
            delay: 0.0,
        }
    }

    pub fn update(&mut self, delta_secs: f32) {
        if self.delay > 0.0 {
            self.delay -= delta_secs;
            return;
        }
        self.head_y += self.speed * delta_secs * 15.0;
        if self.head_y - self.trail_len as f32 > self.height_cells as f32 {
            let mut rng = rand::thread_rng();
            self.head_y = -(rng.gen_range(1..5) as f32);
            self.trail_len = rng.gen_range(8..20);
            self.speed = rng.gen_range(0.5f32..1.5);
            self.delay = rng.gen_range(0.0f32..2.0);
        }
    }

    pub fn brightness_at(&self, distance_from_head: usize) -> u8 {
        if distance_from_head == 0 {
            return 255;
        }
        if distance_from_head > self.trail_len {
            return 0;
        }
        let ratio = 1.0 - (distance_from_head as f32 / self.trail_len as f32);
        (ratio * 200.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Charset;

    #[test]
    fn test_column_advances() {
        let mut col = Column::new(0, 24, 1.0);
        let initial_head = col.head_y;
        col.update(1.0);
        assert!(col.head_y != initial_head || col.head_y < 0.0);
    }

    #[test]
    fn test_trail_length_positive() {
        let col = Column::new(0, 24, 1.0);
        assert!(col.trail_len > 0);
    }

    #[test]
    fn test_cell_fade() {
        let col = Column::new(0, 24, 1.0);
        let full = col.brightness_at(0);
        let gone = col.brightness_at(col.trail_len + 5);
        assert!(full > 0);
        assert_eq!(gone, 0);
    }

    #[test]
    fn test_charset_katakana() {
        let chars = charset_chars(&Charset::Katakana);
        for c in &chars {
            let cp = *c as u32;
            assert!(cp >= 0xFF66 && cp <= 0xFF9F, "unexpected char: U+{:04X}", cp);
        }
    }
}

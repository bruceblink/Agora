use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use common::dto::CaptchaDTO;
use image::{ImageBuffer, Rgb, RgbImage};
use rand::Rng;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CAPTCHA_TTL: Duration = Duration::from_secs(120);
const IMAGE_WIDTH: u32 = 160;
const IMAGE_HEIGHT: u32 = 60;
const GLYPH_WIDTH: u32 = 5;
const GLYPH_HEIGHT: u32 = 7;
const GLYPH_SCALE: u32 = 4;
const GLYPH_SPACING: u32 = 3;
const KEYSTONE_CAPTCHA_CODE_WRONG: i32 = 10203;
const KEYSTONE_CAPTCHA_CODE_EXPIRE: i32 = 10204;

#[derive(Clone, Default)]
pub struct CaptchaStore {
    entries: Arc<Mutex<HashMap<String, CaptchaEntry>>>,
}

struct CaptchaEntry {
    answer: String,
    expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaValidationError {
    Wrong,
    Expired,
}

impl CaptchaValidationError {
    pub fn code(self) -> i32 {
        match self {
            Self::Wrong => KEYSTONE_CAPTCHA_CODE_WRONG,
            Self::Expired => KEYSTONE_CAPTCHA_CODE_EXPIRE,
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::Wrong => "验证码错误",
            Self::Expired => "验证码过期",
        }
    }
}

impl CaptchaStore {
    pub fn insert(&self, answer: String) -> String {
        let key = format!("{:032x}", rand::random::<u128>());
        let mut entries = self.lock_entries();
        cleanup_expired(&mut entries);
        entries.insert(
            key.clone(),
            CaptchaEntry {
                answer,
                expires_at: Instant::now() + CAPTCHA_TTL,
            },
        );
        key
    }

    pub fn validate(
        &self,
        captcha_code_key: Option<&str>,
        captcha_code: Option<&str>,
    ) -> Result<(), CaptchaValidationError> {
        let Some(key) = captcha_code_key
            .map(str::trim)
            .filter(|key| !key.is_empty())
        else {
            return Err(CaptchaValidationError::Expired);
        };

        let mut entries = self.lock_entries();
        cleanup_expired(&mut entries);
        let Some(entry) = entries.remove(key) else {
            return Err(CaptchaValidationError::Expired);
        };

        let code = captcha_code.unwrap_or_default().trim();
        if code.eq_ignore_ascii_case(&entry.answer) {
            Ok(())
        } else {
            Err(CaptchaValidationError::Wrong)
        }
    }

    fn lock_entries(&self) -> std::sync::MutexGuard<'_, HashMap<String, CaptchaEntry>> {
        self.entries.lock().unwrap_or_else(|err| err.into_inner())
    }
}

fn cleanup_expired(entries: &mut HashMap<String, CaptchaEntry>) {
    let now = Instant::now();
    entries.retain(|_, entry| entry.expires_at > now);
}

pub fn build_captcha_dto(
    is_captcha_on: bool,
    captcha_store: &CaptchaStore,
) -> anyhow::Result<CaptchaDTO> {
    if !is_captcha_on {
        return Ok(CaptchaDTO {
            is_captcha_on: false,
            captcha_code_key: String::new(),
            captcha_code_img: String::new(),
        });
    }

    let challenge = generate_math_challenge();
    let captcha_code_key = captcha_store.insert(challenge.answer);
    let image = render_captcha_image(&challenge.expression)?;

    Ok(CaptchaDTO {
        is_captcha_on: true,
        captcha_code_key,
        captcha_code_img: STANDARD.encode(image),
    })
}

struct MathChallenge {
    expression: String,
    answer: String,
}

fn generate_math_challenge() -> MathChallenge {
    let x = rand::random_range(0..13);
    let y = rand::random_range(0..13);
    let operand = match rand::random_range(0..4) {
        0 => Operand::Add,
        1 => Operand::Minus,
        2 => Operand::Multiple,
        _ => Operand::Divide,
    };
    math_challenge(x, y, operand)
}

#[derive(Clone, Copy)]
enum Operand {
    Add,
    Minus,
    Multiple,
    Divide,
}

fn math_challenge(x: i32, y: i32, operand: Operand) -> MathChallenge {
    let (expression, answer) = match operand {
        Operand::Add => (format!("{x}+{y}=?"), x + y),
        Operand::Minus => {
            let max = x.max(y);
            let min = x.min(y);
            (format!("{max}-{min}=?"), max - min)
        }
        Operand::Multiple => (format!("{x}*{y}=?"), x * y),
        Operand::Divide if x != 0 && y % x == 0 => (format!("{y}/{x}=?"), y / x),
        Operand::Divide => (format!("{x}+{y}=?"), x + y),
    };

    MathChallenge {
        expression,
        answer: answer.to_string(),
    }
}

fn render_captcha_image(expression: &str) -> anyhow::Result<Vec<u8>> {
    let mut image: RgbImage =
        ImageBuffer::from_pixel(IMAGE_WIDTH, IMAGE_HEIGHT, Rgb([250, 252, 255]));
    draw_border(&mut image);
    draw_noise(&mut image);
    draw_expression(&mut image, expression);

    let mut bytes = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85);
    encoder.encode_image(&image)?;
    Ok(bytes.into_inner())
}

fn draw_border(image: &mut RgbImage) {
    let color = Rgb([105, 179, 90]);
    for x in 0..IMAGE_WIDTH {
        image.put_pixel(x, 0, color);
        image.put_pixel(x, IMAGE_HEIGHT - 1, color);
    }
    for y in 0..IMAGE_HEIGHT {
        image.put_pixel(0, y, color);
        image.put_pixel(IMAGE_WIDTH - 1, y, color);
    }
}

fn draw_noise(image: &mut RgbImage) {
    let mut rng = rand::rng();
    for _ in 0..28 {
        let x = rng.random_range(2..IMAGE_WIDTH - 2);
        let y = rng.random_range(2..IMAGE_HEIGHT - 2);
        image.put_pixel(x, y, Rgb([210, 223, 238]));
    }
}

fn draw_expression(image: &mut RgbImage, expression: &str) {
    let glyph_count = expression.chars().count() as u32;
    let text_width =
        glyph_count * GLYPH_WIDTH * GLYPH_SCALE + glyph_count.saturating_sub(1) * GLYPH_SPACING;
    let start_x = IMAGE_WIDTH.saturating_sub(text_width) / 2;
    let start_y = (IMAGE_HEIGHT - GLYPH_HEIGHT * GLYPH_SCALE) / 2;

    let mut x = start_x;
    for ch in expression.chars() {
        draw_glyph(image, ch, x, start_y);
        x += GLYPH_WIDTH * GLYPH_SCALE + GLYPH_SPACING;
    }
}

fn draw_glyph(image: &mut RgbImage, ch: char, left: u32, top: u32) {
    let Some(pattern) = glyph(ch) else {
        return;
    };
    let color = Rgb([24, 75, 188]);
    for (row_idx, row) in pattern.iter().enumerate() {
        for (col_idx, bit) in row.chars().enumerate() {
            if bit != '1' {
                continue;
            }
            let x = left + col_idx as u32 * GLYPH_SCALE;
            let y = top + row_idx as u32 * GLYPH_SCALE;
            fill_block(image, x, y, color);
        }
    }
}

fn fill_block(image: &mut RgbImage, left: u32, top: u32, color: Rgb<u8>) {
    for dx in 0..GLYPH_SCALE {
        for dy in 0..GLYPH_SCALE {
            let x = left + dx;
            let y = top + dy;
            if x < IMAGE_WIDTH && y < IMAGE_HEIGHT {
                image.put_pixel(x, y, color);
            }
        }
    }
}

fn glyph(ch: char) -> Option<[&'static str; 7]> {
    let pattern = match ch {
        '0' => [
            "11111", "10001", "10011", "10101", "11001", "10001", "11111",
        ],
        '1' => [
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ],
        '2' => [
            "11110", "00001", "00001", "11110", "10000", "10000", "11111",
        ],
        '3' => [
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ],
        '4' => [
            "10010", "10010", "10010", "11111", "00010", "00010", "00010",
        ],
        '5' => [
            "11111", "10000", "10000", "11110", "00001", "00001", "11110",
        ],
        '6' => [
            "01111", "10000", "10000", "11110", "10001", "10001", "01110",
        ],
        '7' => [
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ],
        '8' => [
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ],
        '9' => [
            "01110", "10001", "10001", "01111", "00001", "00001", "11110",
        ],
        '+' => [
            "00000", "00100", "00100", "11111", "00100", "00100", "00000",
        ],
        '-' => [
            "00000", "00000", "00000", "11111", "00000", "00000", "00000",
        ],
        '*' => [
            "00000", "10001", "01010", "00100", "01010", "10001", "00000",
        ],
        '/' => [
            "00001", "00010", "00010", "00100", "01000", "01000", "10000",
        ],
        '=' => [
            "00000", "00000", "11111", "00000", "11111", "00000", "00000",
        ],
        '?' => [
            "01110", "10001", "00001", "00010", "00100", "00000", "00100",
        ],
        _ => return None,
    };
    Some(pattern)
}

#[cfg(test)]
mod tests {
    use super::{
        CaptchaStore, CaptchaValidationError, Operand, build_captcha_dto, math_challenge,
        render_captcha_image,
    };

    #[test]
    fn disabled_captcha_response_matches_keystone_shape() {
        let store = CaptchaStore::default();
        let dto = build_captcha_dto(false, &store).unwrap();

        assert!(!dto.is_captcha_on);
        assert!(dto.captcha_code_key.is_empty());
        assert!(dto.captcha_code_img.is_empty());
    }

    #[test]
    fn enabled_captcha_response_generates_key_and_jpeg_image() {
        let store = CaptchaStore::default();
        let dto = build_captcha_dto(true, &store).unwrap();

        assert!(dto.is_captcha_on);
        assert_eq!(dto.captcha_code_key.len(), 32);
        let image_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            dto.captcha_code_img,
        )
        .unwrap();
        assert!(image_bytes.starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn captcha_store_consumes_code_once_and_ignores_case() {
        let store = CaptchaStore::default();
        let key = store.insert("AbC1".to_string());

        assert_eq!(store.validate(Some(&key), Some("abc1")), Ok(()));
        assert_eq!(
            store.validate(Some(&key), Some("abc1")),
            Err(CaptchaValidationError::Expired)
        );
    }

    #[test]
    fn captcha_store_reports_wrong_code() {
        let store = CaptchaStore::default();
        let key = store.insert("8".to_string());

        assert_eq!(
            store.validate(Some(&key), Some("9")),
            Err(CaptchaValidationError::Wrong)
        );
    }

    #[test]
    fn math_captcha_matches_keystone_operand_rules() {
        let minus = math_challenge(2, 9, Operand::Minus);
        assert_eq!(minus.expression, "9-2=?");
        assert_eq!(minus.answer, "7");

        let divide = math_challenge(4, 12, Operand::Divide);
        assert_eq!(divide.expression, "12/4=?");
        assert_eq!(divide.answer, "3");

        let divide_fallback = math_challenge(5, 12, Operand::Divide);
        assert_eq!(divide_fallback.expression, "5+12=?");
        assert_eq!(divide_fallback.answer, "17");
    }

    #[test]
    fn captcha_renderer_returns_jpeg_bytes() {
        let image = render_captcha_image("12+3=?").unwrap();
        assert!(image.starts_with(&[0xFF, 0xD8, 0xFF]));
    }
}

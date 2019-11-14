pub mod rgb {
    #[derive(Copy, Clone, PartialEq, Debug)]
    pub struct Rgb {
        pub red: f32,
        pub green: f32,
        pub blue: f32,
        pub standard: core::marker::PhantomData<f32>,
    }
    pub type LinSrgb = Rgb;
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Alpha<T> {
    pub alpha: f32,
    pub color: T,
}

impl Alpha<rgb::Rgb> {
    pub fn into_components(self) -> (f32, f32, f32, f32) {
        let rgb::Rgb {
            red, green, blue, ..
        } = self.color;
        (red, green, blue, self.alpha)
    }

    pub fn from_components((red, green, blue, alpha): (f32, f32, f32, f32)) -> Self {
        Self {
            color: rgb::Rgb {
                red,
                green,
                blue,
                standard: core::marker::PhantomData,
            },
            alpha,
        }
    }

    pub fn from_raw(_: ()) {}
}

pub struct Hsv {
    pub hue: f64,
    pub saturation: f64,
    pub value: f64,
}

impl Hsv {
    pub fn new(hue: f64, saturation: f64, value: f64) -> Self {
        Self {
            hue,
            saturation,
            value,
        }
    }
}

impl From<LinSrgba> for Hsv {
    fn from(rgba: LinSrgba) -> Self {
        let rgb::Rgb {
            red: r,
            green: g,
            blue: b,
            ..
        } = rgba.color;

        let c_max = r.max(g).max(b);
        let c_min = r.min(g).min(b);
        let delta = c_max - c_min;

        let hue = if delta == 0.0 {
            0.0
        } else if r > g && r > b {
            60.0 * (((g - b) / delta) % 6.0)
        } else if g > r && g > b {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
    }
}

pub struct Hsla {
    hue: f32,
    saturation: f32,
    lightness: f32,
    alpha: f32,
}

impl Hsla {
    pub fn new(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Self {
        Self {
            hue,
            saturation,
            lightness,
            alpha,
        }
    }
}

pub type LinSrgba = Alpha<rgb::LinSrgb>;
pub trait Pixel {}

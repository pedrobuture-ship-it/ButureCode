//! Renderer-independent Buture Glass Dark tokens. Values are logical points.
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b, 255)
    }

    pub fn with_opacity(self, opacity: f32) -> Self {
        Self(
            self.0,
            self.1,
            self.2,
            (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Background {
    pub base: Color,
    pub deep: Color,
    pub editor: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    pub glass: Color,
    pub glass_strong: Color,
    pub panel: Color,
    pub elevated: Color,
    pub floating: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct Border {
    pub subtle: Color,
    pub normal: Color,
    pub focus: Color,
    pub width: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Text {
    pub primary: Color,
    pub secondary: Color,
    pub muted: Color,
    pub disabled: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct Accent {
    pub emerald: Color,
    pub emerald_hover: Color,
    pub emerald_muted: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct Semantic {
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Radius {
    pub small: u8,
    pub panel: u8,
    pub floating: u8,
    pub window: u8,
}
#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub color: Color,
    pub offset: [i8; 2],
    pub blur: u8,
    pub spread: u8,
}
#[derive(Debug, Clone, Copy)]
pub struct Opacity {
    pub subtle: f32,
    pub panel: f32,
    pub elevated: f32,
    pub floating: f32,
    pub modal: f32,
    pub reduced: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Blur {
    pub full: u8,
    pub reduced: u8,
    pub backdrop_size: [usize; 2],
}
#[derive(Debug, Clone, Copy)]
pub struct Motion {
    pub fast: Duration,
    pub normal: Duration,
    pub slow: Duration,
}
#[derive(Debug, Clone, Copy)]
pub struct Typography {
    pub caption: f32,
    pub body: f32,
    pub title: f32,
    pub display: f32,
    pub code: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Syntax {
    pub keyword: Color,
    pub function: Color,
    pub types: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub macros: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Background,
    pub surface: Surface,
    pub border: Border,
    pub text: Text,
    pub accent: Accent,
    pub semantic: Semantic,
    pub spacing: Spacing,
    pub radius: Radius,
    pub shadow: Shadow,
    pub opacity: Opacity,
    pub blur: Blur,
    pub motion: Motion,
    pub typography: Typography,
    pub syntax: Syntax,
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            syntax: Syntax {
                keyword: Color::rgb(185, 160, 221),
                function: Color::rgb(134, 180, 216),
                types: Color::rgb(136, 197, 201),
                string: Color::rgb(155, 194, 163),
                number: Color::rgb(204, 190, 143),
                comment: Color::rgb(112, 138, 127),
                macros: Color::rgb(190, 151, 185),
            },
            background: Background {
                base: Color::rgb(20, 25, 29),
                deep: Color::rgb(12, 17, 21),
                editor: Color::rgb(18, 23, 28),
            },
            surface: Surface {
                glass: Color::rgb(28, 39, 43),
                glass_strong: Color::rgb(30, 42, 46),
                panel: Color::rgb(25, 34, 39),
                elevated: Color::rgb(35, 46, 51),
                floating: Color::rgb(42, 54, 58),
            },
            border: Border {
                subtle: Color(165, 193, 192, 24),
                normal: Color(165, 193, 192, 48),
                focus: Color::rgb(94, 208, 165),
                width: 1.0,
            },
            text: Text {
                primary: Color::rgb(232, 239, 240),
                secondary: Color::rgb(174, 189, 197),
                muted: Color::rgb(126, 148, 160),
                disabled: Color::rgb(87, 107, 118),
            },
            accent: Accent {
                emerald: Color::rgb(100, 220, 176),
                emerald_hover: Color::rgb(131, 233, 194),
                emerald_muted: Color::rgb(38, 75, 64),
            },
            semantic: Semantic {
                success: Color::rgb(100, 220, 176),
                warning: Color::rgb(226, 192, 119),
                error: Color::rgb(222, 137, 139),
                info: Color::rgb(140, 183, 209),
            },
            spacing: Spacing {
                xs: 4.0,
                sm: 8.0,
                md: 12.0,
                lg: 16.0,
                xl: 24.0,
                xxl: 32.0,
            },
            radius: Radius {
                small: 6,
                panel: 10,
                floating: 14,
                window: 16,
            },
            shadow: Shadow {
                color: Color(0, 0, 0, 64),
                offset: [0, 6],
                blur: 20,
                spread: 0,
            },
            opacity: Opacity {
                subtle: 0.50,
                panel: 0.76,
                elevated: 0.84,
                floating: 0.90,
                modal: 0.95,
                reduced: 0.94,
            },
            blur: Blur {
                full: 12,
                reduced: 4,
                backdrop_size: [320, 200],
            },
            motion: Motion {
                fast: Duration::from_millis(100),
                normal: Duration::from_millis(180),
                slow: Duration::from_millis(280),
            },
            typography: Typography {
                caption: 11.0,
                body: 13.0,
                title: 18.0,
                display: 32.0,
                code: 13.0,
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

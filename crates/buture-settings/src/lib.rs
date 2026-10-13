//! Appearance policy independent of the UI toolkit. Persistence belongs to prompt 10.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VisualEffects {
    #[default]
    Full,
    Reduced,
    Off,
}

impl VisualEffects {
    pub const ALL: [Self; 3] = [Self::Full, Self::Reduced, Self::Off];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Reduced => "Reduced",
            Self::Off => "Off",
        }
    }
}

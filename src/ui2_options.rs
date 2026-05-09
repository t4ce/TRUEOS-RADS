use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui2Size {
    pub width: u32,
    pub height: u32,
}

impl Ui2Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl Default for Ui2Size {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ui2ResizeMode {
    None,
    Width,
    Height,
    Both,
}

impl Ui2ResizeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Width => "width",
            Self::Height => "height",
            Self::Both => "both",
        }
    }

    pub fn options() -> Vec<String> {
        ["none", "width", "height", "both"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

impl Default for Ui2ResizeMode {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ui2ScrollbarMode {
    None,
    Horizontal,
    Vertical,
    Both,
    Auto,
}

impl Ui2ScrollbarMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Both => "both",
            Self::Auto => "auto",
        }
    }

    pub fn options() -> Vec<String> {
        ["none", "horizontal", "vertical", "both", "auto"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    pub const fn horizontal_visible(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both | Self::Auto)
    }

    pub const fn vertical_visible(self) -> bool {
        matches!(self, Self::Vertical | Self::Both | Self::Auto)
    }
}

impl Default for Ui2ScrollbarMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ui2VerticalScrollbarSide {
    Left,
    Right,
}

impl Ui2VerticalScrollbarSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    pub const fn vui2_variant(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }

    pub fn options() -> Vec<String> {
        ["left", "right"].into_iter().map(str::to_string).collect()
    }
}

impl Default for Ui2VerticalScrollbarSide {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ui2HorizontalScrollbarSide {
    Top,
    Bottom,
}

impl Ui2HorizontalScrollbarSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }

    pub const fn vui2_variant(self) -> &'static str {
        match self {
            Self::Top => "Top",
            Self::Bottom => "Bottom",
        }
    }

    pub fn options() -> Vec<String> {
        ["top", "bottom"].into_iter().map(str::to_string).collect()
    }
}

impl Default for Ui2HorizontalScrollbarSide {
    fn default() -> Self {
        Self::Bottom
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui2WindowOptions {
    pub min_size: Ui2Size,
    pub max_size: Option<Ui2Size>,
    pub resize_mode: Ui2ResizeMode,
    pub scrollbars: Ui2ScrollbarMode,
    pub vertical_scrollbar_side: Ui2VerticalScrollbarSide,
    pub horizontal_scrollbar_side: Ui2HorizontalScrollbarSide,
    pub hit_test_visible: bool,
    pub preserve_scale: bool,
}

impl Default for Ui2WindowOptions {
    fn default() -> Self {
        Self {
            min_size: Ui2Size::new(320, 240),
            max_size: None,
            resize_mode: Ui2ResizeMode::Both,
            scrollbars: Ui2ScrollbarMode::None,
            vertical_scrollbar_side: Ui2VerticalScrollbarSide::Left,
            horizontal_scrollbar_side: Ui2HorizontalScrollbarSide::Bottom,
            hit_test_visible: true,
            preserve_scale: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui2HtmlCssDescription {
    pub html: String,
    pub css: String,
}

impl Default for Ui2HtmlCssDescription {
    fn default() -> Self {
        Self {
            html: r#"<main class="ui2-window" data-layout="absolute"><section class="ui2-surface"></section></main>"#
                .to_string(),
            css: ".ui2-window { position: relative; width: 100%; height: 100%; overflow: hidden; }\n.ui2-surface { position: relative; min-width: 100%; min-height: 100%; }\n"
                .to_string(),
        }
    }
}

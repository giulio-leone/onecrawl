use clap::Subcommand;


#[derive(Subcommand)]
pub(crate) enum SelectAction {
    /// CSS selector (supports ::text, ::attr(name) pseudo-elements)
    Css {
        /// CSS selector string
        selector: String,
    },
    /// XPath selector
    Xpath {
        /// XPath expression
        expression: String,
    },
    /// Find elements by text content
    Text {
        /// Text to search for
        text: String,
        /// Filter by tag name
        #[arg(long)]
        tag: Option<String>,
    },
    /// Find elements by regex pattern
    Regex {
        /// Regex pattern
        pattern: String,
        /// Filter by tag name
        #[arg(long)]
        tag: Option<String>,
    },
    /// Auto-generate a unique CSS selector for an element
    AutoSelector {
        /// Target CSS selector
        selector: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum NavAction {
    /// Get parent element
    Parent {
        /// CSS selector
        selector: String,
    },
    /// Get child elements
    Children {
        /// CSS selector
        selector: String,
    },
    /// Get next sibling element
    NextSibling {
        /// CSS selector
        selector: String,
    },
    /// Get previous sibling element
    PrevSibling {
        /// CSS selector
        selector: String,
    },
    /// Get all sibling elements
    Siblings {
        /// CSS selector
        selector: String,
    },
    /// Find similar elements
    Similar {
        /// CSS selector
        selector: String,
    },
    /// Get elements above the target
    Above {
        /// CSS selector
        selector: String,
        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Get elements below the target
    Below {
        /// CSS selector
        selector: String,
        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}


// ── New sub-enums for agent-browser feature parity ──────────────────

#[derive(Subcommand)]
pub(crate) enum KeyboardAction {
    /// Type text with real keystrokes at current focus (no selector)
    Type {
        /// Text to type
        text: String,
    },
    /// Insert text without key events at current focus
    InsertText {
        /// Text to insert
        text: String,
    },
}


#[derive(Subcommand)]
pub(crate) enum MouseAction {
    /// Move mouse to coordinates
    Move {
        /// X coordinate
        x: f64,
        /// Y coordinate
        y: f64,
    },
    /// Press mouse button
    Down {
        /// Button: left, right, middle
        #[arg(default_value = "left")]
        button: String,
    },
    /// Release mouse button
    Up {
        /// Button: left, right, middle
        #[arg(default_value = "left")]
        button: String,
    },
    /// Scroll wheel
    Wheel {
        /// Vertical delta (positive = down)
        dy: f64,
        /// Horizontal delta (positive = right)
        #[arg(default_value = "0")]
        dx: f64,
    },
}


#[derive(Subcommand)]
pub(crate) enum FindAction {
    /// Find by ARIA role and perform action
    Role {
        /// ARIA role (button, link, textbox, heading, etc.)
        role: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
        /// Filter by accessible name
        #[arg(long)]
        name: Option<String>,
        /// Require exact text match
        #[arg(long)]
        exact: bool,
    },
    /// Find by text content and perform action
    Text {
        /// Text to search for
        text: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
        /// Require exact text match
        #[arg(long)]
        exact: bool,
    },
    /// Find by associated label and perform action
    Label {
        /// Label text
        label: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
    },
    /// Find by placeholder text and perform action
    Placeholder {
        /// Placeholder text
        placeholder: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
    },
    /// Find by alt text and perform action
    Alt {
        /// Alt text
        alt: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
    },
    /// Find by title attribute and perform action
    Title {
        /// Title text
        title: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
    },
    /// Find by data-testid attribute and perform action
    TestId {
        /// data-testid value
        testid: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
    },
    /// Find first matching element and perform action
    First {
        /// CSS selector
        selector: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
    },
    /// Find last matching element and perform action
    Last {
        /// CSS selector
        selector: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
    },
    /// Find nth matching element and perform action
    Nth {
        /// Zero-based index
        n: usize,
        /// CSS selector
        selector: String,
        /// Action: click, fill, type, hover, focus, check, uncheck, text
        action: String,
        /// Value for fill/type actions
        value: Option<String>,
    },
}


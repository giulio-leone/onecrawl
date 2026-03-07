# AI Agent Auto Infrastructure Report
## onecrawl-rust Package Analysis

---

## 1. AGENT LOOP IMPLEMENTATION

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/agent.rs` (564 lines)

### Agent Loop Structure
The foundational agent loop implements an **Observe → Verify → Loop** pattern:

```rust
pub async fn agent_loop(
    page: &Page,
    goal: &str,
    max_steps: usize,
    verify_js: Option<&str>,
) -> Result<Value>
```

**Core Loop (lines 11-86)**:
- **Observe Phase**: Extracts page state (URL, title, interactive elements count)
  - Counts total interactive elements (links, buttons, inputs, selectors, etc.)
  - Tracks visible vs. hidden elements
  - Monitors form count and body text length

- **Verify Phase**: Checks if goal is achieved via optional JavaScript verification
  - Executes `verify_js` expression (returns true/false)
  - Captures verification result for diagnostic logging

- **Termination**: Returns structured result with:
  - `status`: "goal_achieved" | "max_steps_reached"
  - `total_steps`: Number of iterations completed
  - `steps[]`: Array of step observations with URL, title, observation data

### Related Functions
- **`goal_assert()`** (lines 88-157): Semantic verification with multiple assertion types:
  - `url_contains`, `url_equals`
  - `title_contains`, `title_equals`
  - `element_exists`, `text_contains`, `element_visible`
  - Returns JSON with all assertion results and context

---

## 2. TASK PLANNER IMPLEMENTATION

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/task_planner.rs` (469 lines)

### Goal Decomposition Architecture

**Main Function**:
```rust
pub fn plan_from_goal(goal: &str, context: &HashMap<String, String>) -> TaskPlan
```

### Key Data Structures

**TaskPlan** (lines 11-19):
```rust
pub struct TaskPlan {
    pub goal: String,
    pub steps: Vec<PlannedStep>,        // Decomposed steps
    pub strategy: PlanStrategy,          // Direct | Exploratory | MemoryAssisted | Hybrid
    pub estimated_duration_ms: u64,
    pub confidence: f64,                 // 0.7-1.0 based on context
    pub context_used: Vec<String>,       // Which context keys influenced plan
}
```

**PlannedStep** (lines 22-29):
```rust
pub struct PlannedStep {
    pub id: usize,
    pub description: String,
    pub action: PlannedAction,
    pub fallback: Option<Box<PlannedStep>>,  // Automatic fallback strategy
    pub confidence: f64,
}
```

**PlannedAction Enum** (lines 32-49):
```rust
pub enum PlannedAction {
    Navigate { url: String },
    Click { target: String, strategy: String },
    Type { target: String, text: String, strategy: String },
    Wait { target: String, timeout_ms: u64 },
    Snapshot {},
    Extract { target: String },
    Assert { condition: String },
    SmartClick { query: String },           // Multi-strategy element matching
    SmartFill { query: String, value: String },
    Scroll { direction: String, amount: Option<u32> },
    Screenshot { path: Option<String> },
    MemoryStore { key: String, value: String },
    MemoryRecall { key: String },
    Conditional { condition: String, then_step: Box<PlannedStep>, else_step: Option<Box<PlannedStep>> },
}
```

### Built-in Goal Patterns (lines 130-206)
The planner includes **7 default patterns**:
1. **Authentication**: login, sign in, authenticate
2. **Search**: search, find, look for, query
3. **Data Extraction**: extract, scrape, get data, collect
4. **Form Filling**: fill, form, submit, complete
5. **Navigation**: navigate, go to, open, visit
6. **Interaction**: click, press, tap, select
7. **Monitoring**: monitor, watch, check, track

Each pattern has **template steps** with confidence levels (0.6-0.9).

### Goal Matching Algorithm (lines 208-234)
```rust
pub fn match_goal(goal: &str) -> (GoalCategory, Vec<StepTemplate>)
```
- Uses **keyword scoring** (counts matches across all patterns)
- Returns best-matching pattern + fallback to "Generic" (3-step default)

### Context Extraction (lines 318-362)
Automatically detects:
- URLs (http:// or https://)
- Quoted values ("..." or '...')
- Email addresses (name@domain.com)

### Planning Strategy Selection (lines 300-306)
- **Direct**: When goal matches a known pattern
- **Exploratory**: For generic/unknown goals
- **MemoryAssisted**: When domain strategy exists
- **Hybrid**: When mixing multiple approaches

---

## 3. COMPUTER USE MODULE

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/computer_use.rs` (356 lines)

### Observation-Action Protocol

**Observation Structure** (lines 12-30):
```rust
pub struct Observation {
    pub url: String,
    pub title: String,
    pub snapshot: String,               // Accessibility snapshot (compact)
    pub interactive_count: usize,
    pub screenshot: Option<String>,     // Base64-encoded PNG
    pub last_error: Option<String>,
    pub cursor: Option<(f64, f64)>,
    pub viewport: Viewport,
}
```

**AgentAction Enum** (lines 38-107) - Comprehensive action set:
- **Navigation**: `Navigate { url }`
- **Interaction**: `Click { target, button }`, `Type { text }`, `Key { key }`
- **Scrolling**: `Scroll { x, y, delta_x, delta_y }`
- **Observation**: `Screenshot`, `Observe { include_screenshot }`
- **Evaluation**: `Evaluate { expression }`
- **Form Ops**: `Fill { selector, value }`, `Select { selector, value }`
- **Advanced**: `Drag { from_x, from_y, to_x, to_y }`
- **Terminal**: `Done { result }`, `Fail { reason }`

### Action Dispatch Pattern (lines 133-285)

**Function**:
```rust
pub async fn execute_action(
    page: &Page,
    action: &AgentAction,
    action_index: usize,
) -> Result<ActionResult>
```

**Execution Flow**:
1. **Coordinate-based click**: Uses `document.elementFromPoint(x, y)?.click()`
2. **Selector-based click**: Resolves ref ID or CSS selector via `element::click()`
3. **Type**: Appends to `document.activeElement.value` with input event
4. **Key press**: Dispatches KeyboardEvent (keydown + keyup)
5. **Scroll**: Calls `window.scrollBy(delta_x, delta_y)`
6. **Navigate**: Uses `crate::navigation::goto()`
7. **Fill**: Calls `element::type_text()` with keyboard events
8. **Drag**: Simulates mousedown → mousemove → mouseup sequence

**ActionResult** (lines 125-131):
```rust
pub struct ActionResult {
    pub success: bool,
    pub observation: Observation,       // Updated page state after action
    pub action_index: usize,
    pub elapsed_ms: u64,
}
```

### Observation Generation (lines 287-355)

**Function**:
```rust
pub async fn observe(
    page: &Page,
    last_error: Option<String>,
    include_screenshot: bool,
) -> Result<Observation>
```

**Data Collection**:
- Extracts URL via `window.location.href`
- Gets title via `document.title`
- **Accessibility snapshot** (via `crate::accessibility::agent_snapshot()`)
  - Compact snapshot (depth limited to 10 levels)
  - Interactive elements count
  - Reference IDs for element targeting
- **Screenshot** (if requested):
  - PNG viewport capture (base64-encoded)
  - Uses `crate::screenshot::screenshot_viewport()`
- **Viewport dimensions**: From `window.innerWidth/Height`

---

## 4. SMART ACTIONS IMPLEMENTATION

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/smart_actions.rs` (193 lines)

### Multi-Strategy Element Resolution

**SmartMatch Structure** (lines 10-17):
```rust
pub struct SmartMatch {
    pub selector: String,
    pub confidence: f64,                // 0.4-1.0 scale
    pub strategy: String,               // Which strategy matched
    pub ref_id: Option<String>,         // onecrawl internal reference
}
```

### Smart Find Algorithm (lines 19-151)

```rust
pub async fn smart_find(page: &Page, query: &str) -> Result<Vec<SmartMatch>>
```

**5-Strategy Cascade** (in order of precedence):

1. **Exact Text Match** (confidence: 1.0)
   - Targets: button, a, [role="button"], [role="link"], input[type="submit"]
   - Compares `textContent || value || aria-label` (case-insensitive)

2. **Fuzzy Text Match** (confidence: 0.5 + similarity*0.3)
   - Broader targets: button, a, label, input, select, textarea
   - Uses substring inclusion: `text.includes(query) || query.includes(text)`
   - Similarity score: min(len)/max(len) ratio

3. **ARIA Role Match** (confidence: 0.6)
   - If query matches ARIA role (button, link, textbox, checkbox, etc.)
   - Queries by role attribute or native element

4. **Attribute Match** (confidence: 0.4 + similarity*0.3)
   - Searches: placeholder, name, id, title, alt, aria-label
   - Uses substring matching with similarity scoring

5. **CSS Selector Direct** (confidence: 0.95)
   - If query starts with `.`, `#`, or contains `[`
   - Attempts direct `document.querySelector(query)`

**Result Deduplication & Ranking**:
- Removes duplicate selectors (seen set)
- Sorts by confidence (descending)
- Returns top 10 matches

### Smart Click (lines 153-172)

```rust
pub async fn smart_click(page: &Page, query: &str) -> Result<SmartMatch>
```

**Steps**:
1. Find matches via `smart_find()`
2. Select best match (highest confidence)
3. Resolve ref ID or use selector directly
4. Execute click via `crate::element::click()`

### Smart Fill (lines 174-193)

```rust
pub async fn smart_fill(page: &Page, query: &str, value: &str) -> Result<SmartMatch>
```

**Steps**:
1. Find matches via `smart_find()`
2. Select best match
3. Resolve selector
4. Type text via `crate::element::type_text()`

---

## 5. ADAPTIVE RETRY FOR FAILURE RECOVERY

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/annotated.rs` (216 lines)

### Adaptive Retry Function (lines 125-216)

```rust
pub async fn adaptive_retry(
    page: &Page,
    action_js: &str,
    max_retries: usize,
    strategies: &[String],
) -> Result<Value>
```

**Retry Algorithm**:
1. **Primary attempt**: Execute main `action_js`
   - Success if result is not "null", "false", or "undefined"

2. **Alternative strategies**: Try up to `max_retries` fallback JS implementations
   - 500ms delay between attempts
   - Each strategy is a different JS snippet

3. **Tracking**: Records all attempts
   ```json
   {
       "status": "success",
       "strategy": "alternative_2",
       "attempt": 3,
       "result": "<result>",
       "attempts": [
           {"strategy": "primary", "success": false, "result": "null"},
           {"strategy": "alternative_1", "success": false, "error": "..."},
           {"strategy": "alternative_2", "success": true, "result": "..."}
       ]
   }
   ```

### Annotated Screenshot for Visual Reasoning (lines 7-123)

```rust
pub async fn annotated_screenshot(page: &Page) -> Result<Value>
```

**Creates visual overlays** on interactive elements:
- Red numbered badges (1, 2, 3, ...) for each interactive element
- Bounding boxes around each element
- Returns base64 PNG + element map with coordinates
- **Element map includes**:
  - Element number, tag, text
  - Bounds: x, y, width, height, center_x, center_y

---

## 6. AGENT MEMORY MODULE

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/agent_memory.rs` (447 lines)

### Memory Architecture

**MemoryEntry** (lines 11-22):
```rust
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub category: MemoryCategory,      // Domain-specific categorization
    pub domain: Option<String>,        // Which domain (github.com, etc.)
    pub created_at: u64,
    pub accessed_at: u64,
    pub access_count: u64,
    pub ttl_seconds: Option<u64>,      // Time-to-live for entries
}
```

**MemoryCategory Enum** (lines 24-35):
```rust
pub enum MemoryCategory {
    PageVisit,                         // URL visit history
    ElementPattern,                    // Learned element selectors
    DomainStrategy,                    // Domain-specific tactics
    RetryKnowledge,                    // Failure recovery patterns
    UserPreference,                    // User-defined data
    SelectorMapping,                   // Element → selector mappings
    ErrorPattern,                      // Common error patterns
    Custom,
}
```

### Storage & Persistence (lines 99-286)

**AgentMemory Struct**:
```rust
pub struct AgentMemory {
    entries: HashMap<String, MemoryEntry>,
    path: PathBuf,                     // JSON file path for persistence
    max_entries: usize,                // LRU limit (default: 10,000)
}
```

**Key Operations**:

1. **Load/Save** (lines 115-137):
   - `load(path)`: Deserialize from JSON file
   - `save()`: Persist to JSON file

2. **Store** (lines 140-167):
   - Inserts or updates entry
   - Tracks created_at and accessed_at timestamps
   - Increments access_count
   - Implements LRU eviction when max_entries reached

3. **Recall** (lines 169-186):
   - Retrieves entry by key
   - Checks TTL and removes if expired
   - Updates accessed_at timestamp
   - Increments access_count

4. **Search** (lines 188-205):
   - `search(query, category, domain)`: Multi-criteria search
   - Matches on key OR value (case-insensitive)
   - Filters by category and domain

5. **Domain Operations** (lines 201-229):
   - `search_by_domain(domain)`: Get all entries for a domain
   - `clear_domain(domain)`: Remove all entries for a domain
   - `forget(key)`: Remove single entry

### Specialized Memory Types

**DomainStrategy** (lines 37-46):
```rust
pub struct DomainStrategy {
    pub domain: String,
    pub login_selectors: Option<LoginSelectors>,
    pub navigation_patterns: Vec<NavigationPattern>,
    pub known_popups: Vec<PopupPattern>,
    pub rate_limit_info: Option<RateLimitInfo>,
    pub anti_bot_level: Option<String>,
}
```

**PageVisit** (lines 77-86):
```rust
pub struct PageVisit {
    pub url: String,
    pub title: Option<String>,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub actions_taken: Vec<String>,
    pub success: bool,
}
```

**ElementPattern** (lines 88-97):
```rust
pub struct ElementPattern {
    pub domain: String,
    pub description: String,
    pub primary_selector: String,
    pub fallback_selectors: Vec<String>,
    pub success_count: u64,
    pub failure_count: u64,
}
```

### Memory Statistics (lines 231-246)

```rust
pub fn stats(&self) -> MemoryStats
```

Returns:
- Total entries count
- Breakdown by category
- Breakdown by domain
- Max entries limit

---

## 7. SAFETY POLICY MODULE

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/safety.rs` (441 lines)

### Safety Policy Definition

**SafetyPolicy Struct** (lines 9-39):
```rust
pub struct SafetyPolicy {
    pub allowed_domains: Vec<String>,          // Whitelist (empty = allow all)
    pub blocked_domains: Vec<String>,          // Blacklist (takes precedence)
    pub blocked_url_patterns: Vec<String>,     // Glob patterns (with * wildcards)
    pub max_actions: usize,                    // Session action limit (0 = unlimited)
    pub confirm_form_submit: bool,             // Require operator confirmation
    pub confirm_file_upload: bool,             // Require operator confirmation
    pub blocked_commands: Vec<String>,         // Blacklisted commands
    pub allowed_commands: Vec<String>,         // Whitelist (empty = allow all)
    pub rate_limit_per_minute: usize,          // Action rate limit (0 = unlimited)
}
```

### Safety Checks Enum (lines 49-58)

```rust
pub enum SafetyCheck {
    Allowed,
    Denied(String),                    // Explains why denied
    RequiresConfirmation(String),      // Operator approval needed
}
```

### SafetyState Runtime Tracking (lines 41-192)

**Structure**:
```rust
pub struct SafetyState {
    policy: SafetyPolicy,
    action_count: usize,               // Total actions this session
    actions_this_minute: usize,        // Rate limit window counter
    minute_start: Instant,             // Rate window timestamp
}
```

**Enforcement Functions**:

1. **URL Check** (lines 80-118):
   - Extracts domain from URL
   - Checks blocked domains (precedence)
   - Checks blocked URL patterns (glob matching)
   - Checks allowed domains (if list non-empty)
   - Returns `Allowed` or `Denied(reason)`

2. **Command Check** (lines 120-157):
   - Verifies action count not exceeded
   - Checks blocked commands
   - Checks allowed commands (if list non-empty)
   - Returns confirmation requirement for destructive ops:
     - `confirm_form_submit` → fill_form
     - `confirm_file_upload` → upload_file

3. **Rate Limit Check** (lines 159-179):
   - Tracks per-minute action count
   - Resets window after 60 seconds
   - Returns `Denied` when limit exceeded

4. **Action Recording** (lines 181-192):
   - Increments `action_count`
   - Increments `actions_this_minute`
   - Handles minute window rollover

### Cost Tracking via Stats (lines 201-218)

```rust
pub fn stats(&self) -> serde_json::Value {
    serde_json::json!({
        "action_count": self.action_count,
        "actions_this_minute": self.actions_this_minute,
        "minute_window_elapsed_secs": elapsed_secs,
        "max_actions": self.policy.max_actions,
        "rate_limit_per_minute": self.policy.rate_limit_per_minute,
        // ... domain and command lists
    })
}
```

### Policy File Loading (lines 194-199)

```rust
pub fn load_from_file(path: &std::path::Path) -> Result<SafetyPolicy, String>
```

Loads policy from JSON file for centralized configuration.

---

## 8. AGENT-IN-THE-LOOP WORKFLOW SUPPORT

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cdp/src/workflow.rs` (907 lines)

### Workflow DSL with Agent Steps

**Workflow Structure** (lines 12-24):
```rust
pub struct Workflow {
    pub name: String,
    pub description: String,
    pub version: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub steps: Vec<Step>,
    pub on_error: ErrorHandler,
}
```

**Step Definition** (lines 26-46):
```rust
pub struct Step {
    pub id: String,
    pub name: String,
    pub action: Action,
    pub condition: Option<String>,     // Conditional execution
    pub retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_ms: Option<u64>,
    pub on_error: Option<StepErrorAction>,
    pub save_as: Option<String>,       // Store result in variable
}
```

**Action Types** (lines 48-71):
```rust
pub enum Action {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    WaitForSelector { selector: String, timeout_ms: u64 },
    Screenshot { path: Option<String>, full_page: Option<bool> },
    Evaluate { js: String },
    Extract { selector: String, attribute: Option<String> },
    SmartClick { query: String },
    SmartFill { query: String, value: String },
    Sleep { ms: u64 },
    SetVariable { name: String, value: serde_json::Value },
    Log { message: String, level: Option<String> },
    Assert { condition: String, message: Option<String> },
    Loop { items: LoopSource, variable: String, steps: Vec<Step> },
    Conditional { condition: String, then_steps: Vec<Step>, else_steps: Option<Vec<Step>> },
    SubWorkflow { path: String },
    HttpRequest { /* ... */ },
    Snapshot { compact: bool, interactive_only: bool },
    Agent { prompt: String, options: Option<Vec<String>> },  // ← KEY FOR PAUSE/RESUME
}
```

### Pause/Resume Mechanism

**AgentStepContext** (lines 168-176):
```rust
pub struct AgentStepContext {
    pub step_index: usize,
    pub prompt: String,                // Question/task for agent
    pub options: Vec<String>,          // Predefined choices
    pub url: String,                   // Current page URL
    pub variables: HashMap<String, serde_json::Value>,  // Current variables
}
```

**AgentDecision** (lines 178-184):
```rust
pub struct AgentDecision {
    pub choice: String,                // Agent's selected option
    pub reasoning: Option<String>,     // Explanation from agent
    pub updates: Option<HashMap<String, serde_json::Value>>,  // Variable updates
}
```

### Workflow Execution with Pause (lines 330-445)

**Execution Logic**:

1. **Agent Step Detection** (lines 378-400):
   ```rust
   if matches!(&step.action, Action::Agent { .. }) {
       // Pause workflow
       let agent_context = output.clone();
       return Ok(WorkflowResult {
           status: StepStatus::Paused,
           paused_at: Some(i),
           agent_context,
           // ... other fields
       });
   }
   ```

2. **Pause Signal** (lines 378-400):
   - When `Action::Agent` is encountered, workflow pauses
   - Returns `WorkflowResult` with:
     - `status: StepStatus::Paused`
     - `paused_at: Some(step_index)`
     - `agent_context`: The agent step's output (prompt + current state)

3. **Resume Pattern**:
   - External agent (Claude, GPT-4, etc.) receives `AgentStepContext`
   - Agent makes decision and returns `AgentDecision`
   - System updates variables with agent's updates
   - Workflow continues from `paused_at + 1`

### WorkflowResult Structure (lines 130-142)

```rust
pub struct WorkflowResult {
    pub name: String,
    pub status: StepStatus,            // Success, Failed, Skipped, or Paused
    pub steps: Vec<StepResult>,
    pub variables: HashMap<String, serde_json::Value>,
    pub total_duration_ms: u64,
    pub steps_succeeded: usize,
    pub steps_failed: usize,
    pub steps_skipped: usize,
    pub paused_at: Option<usize>,      // Which step paused execution
    pub agent_context: Option<serde_json::Value>,  // For agent decision-making
}
```

---

## 9. CLI AGENT COMMANDS

### Core Location
**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cli-rs/src/cli/agent.rs` (51 lines)

### Available Agent Subcommands

**AgentCliAction Enum**:

1. **Loop** (lines 5-15)
   ```
   onecrawl agent loop <GOAL>
   --max-steps <N>              (default: 10)
   --verify <JS_EXPRESSION>     (optional: JS that returns "true" when goal met)
   ```
   - Runs autonomous observe-verify loop
   - Continues until goal_assert passes or max_steps reached

2. **GoalAssert** (lines 16-23)
   ```
   onecrawl agent goal-assert --assertion-type <TYPE> <VALUE>
   ```
   - Types: url_contains, title_contains, element_exists, text_contains, element_visible
   - Validates if assertion passes

3. **Observe** (lines 24-25)
   ```
   onecrawl agent observe
   ```
   - Returns annotated page observation with element coordinates

4. **Context** (lines 26-36)
   ```
   onecrawl agent context set --key <KEY> --value <VALUE>
   onecrawl agent context get --key <KEY>
   onecrawl agent context get_all
   onecrawl agent context clear
   ```
   - Manages session context (window.__onecrawl_ctx)

5. **Chain** (lines 37-48)
   ```
   onecrawl agent chain <JS1> <JS2> ... <JSN>
   --on-error <skip|retry|abort>  (default: skip)
   --retries <N>                  (default: 2)
   ```
   - Executes sequence of JS actions with error recovery

6. **Think** (lines 49-51)
   ```
   onecrawl agent think
   ```
   - Analyzes page and recommends next actions

---

### Computer Use CLI Commands

**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-cli-rs/src/cli/computer.rs` (36 lines)

**ComputerCliAction Enum**:

1. **AnnotatedScreenshot** (lines 5-10)
   ```
   onecrawl computer annotated-screenshot [OUTPUT_FILE]
   ```
   - Takes screenshot with numbered overlays
   - Returns element coordinates map

2. **AdaptiveRetry** (lines 11-21)
   ```
   onecrawl computer adaptive-retry <PRIMARY_JS>
   --alt <ALT1> --alt <ALT2> ...
   --retries <N>  (default: 3)
   ```
   - Tries primary action, then alternatives on failure

3. **ClickAt** (lines 22-28)
   ```
   onecrawl computer click-at <X> <Y>
   ```
   - Clicks at viewport coordinates

4. **MultiPageSync** (lines 29-30)
   ```
   onecrawl computer multi-page-sync
   ```
   - Gets state from all browser tabs

5. **InputReplay** (lines 31-36)
   ```
   onecrawl computer input-replay <EVENTS_FILE.json>
   ```
   - Replays recorded input event sequence

---

## 10. MCP HANDLER INTEGRATION

### Computer Use Handler

**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-mcp-rs/src/handlers/computer.rs`

**Functions**:
- `computer_act()`: Execute single action (returns ActionResult)
- `computer_observe()`: Get current observation
- `computer_batch()`: Execute multiple actions with optional screenshot batching

### Agent Handler

**File**: `/Users/giulioleone/Sviluppo/onecrawl-dev/packages/onecrawl-rust/crates/onecrawl-mcp-rs/src/handlers/agent.rs`

**Functions**:
- `agent_execute_chain()`: Run sequence of commands with error control
- `agent_element_screenshot()`: Capture bounds + screenshot of specific element

---

## 11. MODULE EXPORTS

**onecrawl-cdp lib.rs** exports:
```rust
pub mod agent;              // Agent loop functions
pub mod agent_memory;       // Persistent agent memory
pub mod computer_use;       // Computer use protocol
pub mod smart_actions;      // Smart element matching
pub mod annotated;          // Annotated screenshots + adaptive retry
pub mod task_planner;       // Goal decomposition
pub mod workflow;           // Workflow DSL + pause/resume
pub mod safety;             // Safety policy enforcement
```

---

## IMPLEMENTATION CHECKLIST FOR AI AGENT AUTO

### Core Loops
- ✅ Agent loop (observe → verify → repeat)
- ✅ Task planner (goal → steps)
- ✅ Computer use protocol (action dispatch)
- ✅ Smart actions (multi-strategy element matching)

### Memory & State
- ✅ Agent memory (persistent cross-session)
- ✅ Domain-specific strategies
- ✅ Element pattern learning
- ✅ Session context

### Safety & Control
- ✅ Safety policy (domain/command allowlists)
- ✅ Rate limiting (per-minute caps)
- ✅ Action cost tracking
- ✅ Destructive action confirmation

### Human-in-the-Loop
- ✅ Workflow pause/resume at agent steps
- ✅ AgentStepContext (prompt + options + state)
- ✅ AgentDecision (choice + reasoning + updates)
- ✅ Variable interpolation & condition evaluation

### CLI & Integration
- ✅ Agent CLI commands (loop, think, observe, context, chain)
- ✅ Computer use CLI (annotated-screenshot, adaptive-retry, click-at)
- ✅ MCP handlers (computer_act, computer_observe, agent_execute_chain)

### Failure Recovery
- ✅ Adaptive retry with alternative strategies
- ✅ Fallback steps in task plan
- ✅ Retry knowledge in memory (RetryKnowledge category)
- ✅ Error pattern tracking


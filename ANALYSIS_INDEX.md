# OneCrawl Rust Codebase - Analysis Index

## DRY Violations & Code Duplication Analysis

**Date**: March 7, 2025  
**Scope**: onecrawl-rust crates (handlers, CLI, dispatch)  
**Analysis Type**: Static code duplication and DRY principle violations

### Main Report
📄 **File**: `DRY_VIOLATIONS_ANALYSIS.md` (14 KB)

Contains:
- Executive summary with 670+ duplication instances
- 6 detailed violation patterns with code examples
- Before/after refactoring suggestions
- Priority recommendations
- Impact analysis

### Quick Summary

#### 🔴 HIGH SEVERITY (2 violations)
1. **Handler Boilerplate** (151+ instances)
   - Pattern: `ensure_page() → .mcp()? → text_ok()`
   - Files: All handler files
   - Example: navigation_back/forward/reload are 91% identical

2. **Selector Resolution** (18 instances)
   - Pattern: `onecrawl_cdp::accessibility::resolve_ref()`
   - File: `/commands/dispatch.rs` (lines 43-63+)
   - Impact: Could reduce by 70% with macro

#### 🟡 MEDIUM SEVERITY (4 violations)
3. **Page Evaluation Chains** (12 instances)
   - Location: `/handlers/computer.rs`
   - Pattern: `page.evaluate().await.mcp()?.into_value().unwrap_or()`

4. **Session Connections** (8 instances)
   - Pattern: `BrowserSession::connect()` scattered across files
   - Fix: Extract `connect_session()` helper

5. **Error Handling** (432+ instances)
   - Redundant `.map_err()` patterns across all handlers
   - Audit needed for `.mcp()` trait consistency

6. **Closure Patterns** (40+ instances)
   - Location: `/commands/browser/monitoring.rs`, `/emulation.rs`
   - Pattern: Repetitive `with_page()` closures

### Files to Refactor (Priority)

| Priority | File | Issues | Status |
|----------|------|--------|--------|
| P1 | `/handlers/browser.rs` | 60+ boilerplate | Pending |
| P2 | `/handlers/computer.rs` | 12 eval chains | Pending |
| P3 | `/commands/dispatch.rs` | 18 selector resolutions | Pending |
| P4 | `/helpers.rs` | Central location for fixes | Pending |
| P5 | `/session/core.rs` | 8 connections | Pending |
| P6 | `/handlers/agent.rs` | 30+ boilerplate + 54 errors | Pending |

### Quick Wins (Implementation Order)

1. **PageEvalExt Trait** (20 min)
   - Eliminates 12 chains
   - Location: `helpers.rs`
   
2. **simple_nav_op! Macro** (30 min)
   - Consolidates back/forward/reload
   - Location: `handlers/mod.rs`

3. **Selector Resolution Macro** (45 min)
   - Handles dispatch.rs duplicates
   - Location: `dispatch.rs`

4. **connect_session() Helper** (20 min)
   - Centralizes connections
   - Location: `session/core.rs`

### Impact Estimates

- **Quick Wins**: 150-200 LOC reduction
- **Full Refactoring**: 300-400 LOC reduction (20-30% handler code)
- **Maintenance Improvement**: 25-30% reduction
- **Time to Implement**: 2-3 hours for quick wins

### Key Findings

✅ **Good News**:
- Existing infrastructure is well-designed (ensure_page, .mcp() trait, with_page)
- Violations are mechanical, not architectural
- Easy fixes available with significant payoff
- No breaking changes needed

❌ **Areas for Improvement**:
- Copy-paste patterns dominant
- Inconsistent error handling approach
- Opportunity for macro/trait consolidation

### Analysis Methodology

1. Searched for repeated patterns in:
   - Handler function signatures and bodies
   - Error handling patterns
   - CDP API call patterns (evaluate, call_method)
   - Session initialization code
   - Route handler patterns

2. Counted occurrences of:
   - `ensure_page()` calls
   - `.mcp()` error conversions
   - `onecrawl_cdp::accessibility::resolve_ref()` calls
   - Page evaluation chains
   - Session connection patterns

3. Analyzed 87 files across multiple crates

### Next Steps

1. Review `DRY_VIOLATIONS_ANALYSIS.md` in detail
2. Select one quick win to implement first
3. Create PR with first fix
4. Measure impact (LOC saved, code clarity improvement)
5. Roll out remaining fixes incrementally
6. Update team code style guide

---

**Total Analysis Time**: ~2 hours  
**Detailed Report**: DRY_VIOLATIONS_ANALYSIS.md  
**Questions**: Refer to specific file:line numbers in main report

//! Myers-based line-level unified diff for accessibility snapshots.
//!
//! Compares two text snapshots (from `agent_snapshot`) and produces a
//! unified diff with +/- prefixes plus summary statistics.

use serde::{Deserialize, Serialize};

/// Result of diffing two accessibility snapshot texts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySnapshotDiff {
    /// Unified diff text with +/- prefixes
    pub diff: String,
    /// Number of lines added
    pub additions: usize,
    /// Number of lines removed
    pub removals: usize,
    /// Number of lines unchanged
    pub unchanged: usize,
    /// Whether any changes were detected
    pub changed: bool,
}

/// Compute a line-level unified diff between two accessibility snapshot texts.
///
/// Uses a Myers-style shortest-edit-script (SES) algorithm over lines.
/// Returns a `AccessibilitySnapshotDiff` with the unified diff output and statistics.
pub fn diff_snapshots(before: &str, after: &str) -> AccessibilitySnapshotDiff {
    let old_lines: Vec<&str> = before.lines().collect();
    let new_lines: Vec<&str> = after.lines().collect();

    let edits = myers_diff(&old_lines, &new_lines);

    let mut diff = String::new();
    let mut additions: usize = 0;
    let mut removals: usize = 0;
    let mut unchanged: usize = 0;

    for edit in &edits {
        match edit {
            Edit::Equal(line) => {
                diff.push_str(&format!(" {line}\n"));
                unchanged += 1;
            }
            Edit::Delete(line) => {
                diff.push_str(&format!("-{line}\n"));
                removals += 1;
            }
            Edit::Insert(line) => {
                diff.push_str(&format!("+{line}\n"));
                additions += 1;
            }
        }
    }

    let changed = additions > 0 || removals > 0;

    AccessibilitySnapshotDiff {
        diff,
        additions,
        removals,
        unchanged,
        changed,
    }
}

// ─── Myers diff internals ───────────────────────────────────────────

#[derive(Debug)]
enum Edit<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

/// Myers diff algorithm — computes shortest edit script between two line slices.
fn myers_diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Edit<'a>> {
    let n = old.len();
    let m = new.len();
    let max = n + m;

    if max == 0 {
        return Vec::new();
    }

    // v[k] stores the furthest-reaching x on diagonal k.
    // Diagonal k = x - y. We index with offset `max` so k can be negative.
    let sz = 2 * max + 1;
    let mut v = vec![0usize; sz];
    // trace stores a copy of v at each step d for backtracking.
    let mut trace: Vec<Vec<usize>> = Vec::with_capacity(max + 1);

    'outer: for d in 0..=max {
        trace.push(v.clone());
        let d_i = d as isize;
        let mut k = -d_i;
        while k <= d_i {
            let ki = (k + max as isize) as usize;
            let mut x = if k == -d_i
                || (k != d_i && v[ki.wrapping_sub(1)] < v[ki + 1])
            {
                // move down (insert)
                v[ki + 1]
            } else {
                // move right (delete)
                v[ki.wrapping_sub(1)] + 1
            };

            let mut y = (x as isize - k) as usize;

            // follow diagonal (equal lines)
            while x < n && y < m && old[x] == new[y] {
                x += 1;
                y += 1;
            }

            v[ki] = x;

            if x >= n && y >= m {
                break 'outer;
            }

            k += 2;
        }
    }

    // Backtrack through the trace to recover edits.
    let mut edits: Vec<Edit<'a>> = Vec::new();
    let mut x = n;
    let mut y = m;

    for d in (0..trace.len()).rev() {
        let v_d = &trace[d];
        let k = x as isize - y as isize;
        let ki = (k + max as isize) as usize;

        let (prev_k, prev_x);
        if d == 0 {
            prev_k = 0isize;
            prev_x = 0usize;
        } else {
            let d_i = d as isize;
            if k == -d_i
                || (k != d_i && v_d[ki.wrapping_sub(1)] < v_d[ki + 1])
            {
                // came from k+1 (insert)
                prev_k = k + 1;
                prev_x = v_d[ki + 1];
            } else {
                // came from k-1 (delete)
                prev_k = k - 1;
                prev_x = v_d[ki.wrapping_sub(1)];
            }
        }

        let prev_y = (prev_x as isize - prev_k) as usize;

        // Diagonal (equal) lines
        while x > prev_x && y > prev_y {
            x -= 1;
            y -= 1;
            edits.push(Edit::Equal(old[x]));
        }

        if d > 0 {
            if x == prev_x {
                // insert
                y -= 1;
                edits.push(Edit::Insert(new[y]));
            } else {
                // delete
                x -= 1;
                edits.push(Edit::Delete(old[x]));
            }
        }
    }

    edits.reverse();
    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_snapshots() {
        let text = "[e1] button \"Submit\"\n[e2] link \"Home\"\n";
        let result = diff_snapshots(text, text);
        assert!(!result.changed);
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 0);
        assert_eq!(result.unchanged, 2);
    }

    #[test]
    fn completely_different() {
        let before = "line A\nline B\n";
        let after = "line C\nline D\n";
        let result = diff_snapshots(before, after);
        assert!(result.changed);
        assert_eq!(result.additions, 2);
        assert_eq!(result.removals, 2);
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn addition_only() {
        let before = "[e1] button \"Ok\"";
        let after = "[e1] button \"Ok\"\n[e2] link \"New\"";
        let result = diff_snapshots(before, after);
        assert!(result.changed);
        assert_eq!(result.additions, 1);
        assert_eq!(result.removals, 0);
        assert_eq!(result.unchanged, 1);
        assert!(result.diff.contains("+[e2] link \"New\""));
    }

    #[test]
    fn removal_only() {
        let before = "[e1] button \"Ok\"\n[e2] link \"Old\"";
        let after = "[e1] button \"Ok\"";
        let result = diff_snapshots(before, after);
        assert!(result.changed);
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 1);
        assert_eq!(result.unchanged, 1);
        assert!(result.diff.contains("-[e2] link \"Old\""));
    }

    #[test]
    fn modification() {
        let before = "[e1] button \"Submit\"\n[e2] link \"Home\"";
        let after = "[e1] button \"Submit\"\n[e2] link \"Dashboard\"";
        let result = diff_snapshots(before, after);
        assert!(result.changed);
        assert!(result.diff.contains("-[e2] link \"Home\""));
        assert!(result.diff.contains("+[e2] link \"Dashboard\""));
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn empty_inputs() {
        let result = diff_snapshots("", "");
        assert!(!result.changed);
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 0);
    }

    #[test]
    fn before_empty() {
        let result = diff_snapshots("", "new line");
        assert!(result.changed);
        assert_eq!(result.additions, 1);
        assert_eq!(result.removals, 0);
    }

    #[test]
    fn after_empty() {
        let result = diff_snapshots("old line", "");
        assert!(result.changed);
        assert_eq!(result.removals, 1);
        assert_eq!(result.additions, 0);
    }
}

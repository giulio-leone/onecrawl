use axum::extract::{Json, Path, State};

use super::{ApiResult, get_tab_page};
use crate::action::{parse_ref_id, Action, ActionResult};
use crate::snapshot::{
    click_by_index_js, fill_by_index_js, focus_by_index_js, hover_by_index_js,
    select_by_index_js, type_by_index_js,
};
use crate::state::AppState;

pub async fn execute_action(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(action): Json<Action>,
) -> ApiResult<ActionResult> {
    let page = get_tab_page(&state, &tab_id).await?;
    let result = execute_single_action(&page, &action).await;
    Ok(Json(result))
}

pub async fn execute_actions(
    State(state): State<AppState>,
    Path(tab_id): Path<String>,
    Json(actions): Json<Vec<Action>>,
) -> ApiResult<Vec<ActionResult>> {
    let page = get_tab_page(&state, &tab_id).await?;
    let mut results = Vec::with_capacity(actions.len());
    for action in &actions {
        let r = execute_single_action(&page, action).await;
        let failed = !r.success;
        results.push(r);
        if failed {
            break;
        }
    }
    Ok(Json(results))
}

async fn eval_ref_action(
    page: &chromiumoxide::Page,
    ref_id: &str,
    js_fn: impl FnOnce(i64) -> String,
    action_name: &str,
) -> ActionResult {
    let idx = match parse_ref_id(ref_id) {
        Ok(i) => i,
        Err(e) => return ActionResult::err(e),
    };
    match page.evaluate(js_fn(idx)).await {
        Ok(_) => ActionResult::ok(),
        Err(e) => ActionResult::err(format!("{action_name} failed: {e}")),
    }
}

fn execute_single_action<'a>(
    page: &'a chromiumoxide::Page,
    action: &'a Action,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ActionResult> + Send + 'a>> {
    Box::pin(async move {
    match action {
        Action::Click { ref_id } => eval_ref_action(page, ref_id, click_by_index_js, "click").await,
        Action::Type { ref_id, text } => eval_ref_action(page, ref_id, |i| type_by_index_js(i, text), "type").await,
        Action::Fill { ref_id, text } => eval_ref_action(page, ref_id, |i| fill_by_index_js(i, text), "fill").await,
        Action::Hover { ref_id } => eval_ref_action(page, ref_id, hover_by_index_js, "hover").await,
        Action::Focus { ref_id } => eval_ref_action(page, ref_id, focus_by_index_js, "focus").await,
        Action::Select { ref_id, value } => eval_ref_action(page, ref_id, |i| select_by_index_js(i, value), "select").await,
        Action::Press { key, ref_id } => {
            if let Some(rid) = ref_id {
                let idx = match parse_ref_id(rid) {
                    Ok(i) => i,
                    Err(e) => return ActionResult::err(e),
                };
                if let Err(e) = page.evaluate(focus_by_index_js(idx)).await {
                    return ActionResult::err(format!("focus for press failed: {e}"));
                }
            }
            let escaped = key.replace('\'', "\\'");
            let js = format!(
                "document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {{ key: '{escaped}', bubbles: true }})); \
                 document.activeElement.dispatchEvent(new KeyboardEvent('keyup', {{ key: '{escaped}', bubbles: true }}))"
            );
            match page.evaluate(js).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("press failed: {e}")),
            }
        }
        Action::Scroll { ref_id, pixels } => {
            let px = pixels.unwrap_or(300);
            let js = if let Some(rid) = ref_id {
                let idx = match parse_ref_id(rid) {
                    Ok(i) => i,
                    Err(e) => return ActionResult::err(e),
                };
                format!(
                    r#"(() => {{
                        const walker = document.createTreeWalker(document.body||document.documentElement, NodeFilter.SHOW_ELEMENT, null);
                        let node = walker.currentNode; let i = 0;
                        while (node) {{ if (i === {idx}) {{ node.scrollBy(0, {px}); return 'scrolled'; }} i++; node = walker.nextNode(); }}
                        throw new Error('not found');
                    }})()"#,
                    idx = idx,
                    px = px
                )
            } else {
                format!("window.scrollBy(0, {px}); 'scrolled'")
            };
            match page.evaluate(js).await {
                Ok(_) => ActionResult::ok(),
                Err(e) => ActionResult::err(format!("scroll failed: {e}")),
            }
        }
        Action::Wait { time } => {
            tokio::time::sleep(tokio::time::Duration::from_millis(*time)).await;
            ActionResult::ok()
        }
        Action::Batch { actions } => {
            for a in actions {
                let r = execute_single_action(page, a).await;
                if !r.success {
                    return r;
                }
            }
            ActionResult::ok()
        }
    }
    })
}

//! JavaScript evaluation methods for Page.

use onecrawl_protocol::cdp::browser_protocol::page::*;
use onecrawl_protocol::cdp::js_protocol::debugger::GetScriptSourceParams;
use onecrawl_protocol::cdp::js_protocol::runtime::{
    AddBindingParams, CallFunctionOnParams, EvaluateParams, ExecutionContextId, RemoteObjectType,
    ScriptId,
};

use crate::error::Result;
use crate::js::{Evaluation, EvaluationResult};
use crate::utils;

use super::Page;

impl Page {
    pub async fn expose_function(
        &self,
        name: impl Into<String>,
        function: impl AsRef<str>,
    ) -> Result<()> {
        let name = name.into();
        let expression = utils::evaluation_string(function, &["exposedFun", name.as_str()]);

        self.execute(AddBindingParams::new(name)).await?;
        self.execute(AddScriptToEvaluateOnNewDocumentParams::new(
            expression.clone(),
        ))
        .await?;

        // TODO add execution context tracking for frames
        //let frames = self.frames().await?;

        Ok(())
    }

    /// This evaluates strictly as expression.
    ///
    /// Same as `Page::evaluate` but no fallback or any attempts to detect
    /// whether the expression is actually a function. However you can
    /// submit a function evaluation string:
    ///
    /// # Example Evaluate function call as expression
    ///
    /// This will take the arguments `(1,2)` and will call the function
    ///
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let sum: usize = page
    ///         .evaluate_expression("((a,b) => {return a + b;})(1,2)")
    ///         .await?
    ///         .into_value()?;
    ///     assert_eq!(sum, 3);
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn evaluate_expression(
        &self,
        evaluate: impl Into<EvaluateParams>,
    ) -> Result<EvaluationResult> {
        self.inner.evaluate_expression(evaluate).await
    }

    /// Evaluates an expression or function in the page's context and returns
    /// the result.
    ///
    /// In contrast to `Page::evaluate_expression` this is capable of handling
    /// function calls and expressions alike. This takes anything that is
    /// `Into<Evaluation>`. When passing a `String` or `str`, this will try to
    /// detect whether it is a function or an expression. JS function detection
    /// is not very sophisticated but works for general cases (`(async)
    /// functions` and arrow functions). If you want a string statement
    /// specifically evaluated as expression or function either use the
    /// designated functions `Page::evaluate_function` or
    /// `Page::evaluate_expression` or use the proper parameter type for
    /// `Page::execute`:  `EvaluateParams` for strict expression evaluation or
    /// `CallFunctionOnParams` for strict function evaluation.
    ///
    /// If you don't trust the js function detection and are not sure whether
    /// the statement is an expression or of type function (arrow functions: `()
    /// => {..}`), you should pass it as `EvaluateParams` and set the
    /// `EvaluateParams::eval_as_function_fallback` option. This will first
    /// try to evaluate it as expression and if the result comes back
    /// evaluated as `RemoteObjectType::Function` it will submit the
    /// statement again but as function:
    ///
    ///  # Example Evaluate function statement as expression with fallback
    /// option
    ///
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_protocol::cdp::js_protocol::runtime::{EvaluateParams, RemoteObjectType};
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let eval = EvaluateParams::builder().expression("() => {return 42;}");
    ///     // this will fail because the `EvaluationResult` returned by the browser will be
    ///     // of type `Function`
    ///     let result = page
    ///                 .evaluate(eval.clone().build().unwrap())
    ///                 .await?;
    ///     assert_eq!(result.object().r#type, RemoteObjectType::Function);
    ///     assert!(result.into_value::<usize>().is_err());
    ///
    ///     // This will also fail on the first try but it detects that the browser evaluated the
    ///     // statement as function and then evaluate it again but as function
    ///     let sum: usize = page
    ///         .evaluate(eval.eval_as_function_fallback(true).build().unwrap())
    ///         .await?
    ///         .into_value()?;
    ///     # Ok(())
    /// # }
    /// ```
    ///
    /// # Example Evaluate basic expression
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let sum:usize = page.evaluate("1 + 2").await?.into_value()?;
    ///     assert_eq!(sum, 3);
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn evaluate(&self, evaluate: impl Into<Evaluation>) -> Result<EvaluationResult> {
        match evaluate.into() {
            Evaluation::Expression(mut expr) => {
                if expr.context_id.is_none() {
                    expr.context_id = self.execution_context().await?;
                }
                let fallback = expr.eval_as_function_fallback.and_then(|p| {
                    if p {
                        Some(expr.clone())
                    } else {
                        None
                    }
                });
                let res = self.evaluate_expression(expr).await?;

                if res.object().r#type == RemoteObjectType::Function {
                    // expression was actually a function
                    if let Some(fallback) = fallback {
                        return self.evaluate_function(fallback).await;
                    }
                }
                Ok(res)
            }
            Evaluation::Function(fun) => Ok(self.evaluate_function(fun).await?),
        }
    }

    /// Eexecutes a function withinthe page's context and returns the result.
    ///
    /// # Example Evaluate a promise
    /// This will wait until the promise resolves and then returns the result.
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let sum:usize = page.evaluate_function("() => Promise.resolve(1 + 2)").await?.into_value()?;
    ///     assert_eq!(sum, 3);
    ///     # Ok(())
    /// # }
    /// ```
    ///
    /// # Example Evaluate an async function
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let val:usize = page.evaluate_function("async function() {return 42;}").await?.into_value()?;
    ///     assert_eq!(val, 42);
    ///     # Ok(())
    /// # }
    /// ```
    /// # Example Construct a function call
    ///
    /// ```no_run
    /// # use onecrawl_browser::page::Page;
    /// # use onecrawl_browser::error::Result;
    /// # use onecrawl_protocol::cdp::js_protocol::runtime::{CallFunctionOnParams, CallArgument};
    /// # async fn demo(page: Page) -> Result<()> {
    ///     let call = CallFunctionOnParams::builder()
    ///            .function_declaration(
    ///                "(a,b) => { return a + b;}"
    ///            )
    ///            .argument(
    ///                CallArgument::builder()
    ///                    .value(serde_json::json!(1))
    ///                    .build(),
    ///            )
    ///            .argument(
    ///                CallArgument::builder()
    ///                    .value(serde_json::json!(2))
    ///                    .build(),
    ///            )
    ///            .build()
    ///            .unwrap();
    ///     let sum:usize = page.evaluate_function(call).await?.into_value()?;
    ///     assert_eq!(sum, 3);
    ///     # Ok(())
    /// # }
    /// ```
    pub async fn evaluate_function(
        &self,
        evaluate: impl Into<CallFunctionOnParams>,
    ) -> Result<EvaluationResult> {
        self.inner.evaluate_function(evaluate).await
    }

    /// Returns the default execution context identifier of this page that
    /// represents the context for JavaScript execution.
    pub async fn execution_context(&self) -> Result<Option<ExecutionContextId>> {
        self.inner.execution_context().await
    }

    /// Returns the secondary execution context identifier of this page that
    /// represents the context for JavaScript execution for manipulating the
    /// DOM.
    ///
    /// See `Page::set_contents`
    pub async fn secondary_execution_context(&self) -> Result<Option<ExecutionContextId>> {
        self.inner.secondary_execution_context().await
    }

    pub async fn frame_execution_context(
        &self,
        frame_id: FrameId,
    ) -> Result<Option<ExecutionContextId>> {
        self.inner.frame_execution_context(frame_id).await
    }

    pub async fn frame_secondary_execution_context(
        &self,
        frame_id: FrameId,
    ) -> Result<Option<ExecutionContextId>> {
        self.inner.frame_secondary_execution_context(frame_id).await
    }

    /// Evaluates given script in every frame upon creation (before loading
    /// frame's scripts)
    pub async fn evaluate_on_new_document(
        &self,
        script: impl Into<AddScriptToEvaluateOnNewDocumentParams>,
    ) -> Result<ScriptIdentifier> {
        Ok(self.execute(script.into()).await?.result.identifier)
    }

    /// Returns source for the script with given id.
    ///
    /// Debugger must be enabled.
    pub async fn get_script_source(&self, script_id: impl Into<String>) -> Result<String> {
        Ok(self
            .execute(GetScriptSourceParams::new(ScriptId::from(script_id.into())))
            .await?
            .result
            .script_source)
    }
}

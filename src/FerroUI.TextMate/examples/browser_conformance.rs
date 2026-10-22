//! Compiled with wasm-bindgen and executed by `tests/oracle/check-wasm.mjs`.

#[cfg(target_arch = "wasm32")]
mod browser {
    use ferroui_textmate::{initialize_browser_oniguruma, BundledCatalog, Token};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub async fn verify_upstream_corpus(corpus: String) -> std::result::Result<String, JsValue> {
        initialize_browser_oniguruma()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&corpus).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let catalog = BundledCatalog::new().map_err(|e| JsValue::from_str(&e.to_string()))?;
        let registry = catalog
            .registry()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut lines = 0;
        for case in value["cases"]
            .as_array()
            .ok_or_else(|| JsValue::from_str("missing cases"))?
        {
            let scope = case["scope"]
                .as_str()
                .ok_or_else(|| JsValue::from_str("missing scope"))?;
            let grammar = registry
                .load_grammar(scope)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            let mut state = None;
            for (index, line) in case["lines"]
                .as_array()
                .ok_or_else(|| JsValue::from_str("missing lines"))?
                .iter()
                .enumerate()
            {
                let text = line["text"]
                    .as_str()
                    .ok_or_else(|| JsValue::from_str("missing line text"))?;
                let result = grammar
                    .tokenize_line(text, state.as_ref())
                    .map_err(|e| JsValue::from_str(&format!("{scope} line {index}: {e}")))?;
                let expected: Vec<Token> = serde_json::from_value(line["tokens"].clone())
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                if result.tokens != expected
                    || result.rule_stack.depth() as u64
                        != line["depth"].as_u64().unwrap_or(u64::MAX)
                {
                    return Err(JsValue::from_str(&format!("{scope} line {index}: token stream differs\nexpected: {expected:?}\nactual: {:?}", result.tokens)));
                }
                state = Some(result.rule_stack);
                lines += 1;
            }
        }
        Ok(format!(
            "{lines} lines match the upstream token stream through Rust + browser Oniguruma"
        ))
    }
}

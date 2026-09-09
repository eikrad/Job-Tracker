//! Data-driven provider registry (spec §8.1).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProvider {
    ScalewayDeepseek,
    Mistral,
    Gemini,
}

impl LlmProvider {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "scaleway_deepseek" | "scaleway" => Ok(Self::ScalewayDeepseek),
            "mistral" => Ok(Self::Mistral),
            "gemini" => Ok(Self::Gemini),
            other => Err(format!("Unknown LLM provider: {other}")),
        }
    }

    pub fn secret_provider(self) -> &'static str {
        match self {
            Self::ScalewayDeepseek => "scaleway",
            Self::Mistral => "mistral",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AuthStyle {
    Bearer,
    Header(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub enum JsonMode {
    /// OpenAI-style `response_format: { type: "json_schema", ... }` (preferred for Scaleway).
    Schema,
    /// OpenAI/Mistral `response_format: { type: "json_object" }`.
    JsonObject,
    /// Gemini `generationConfig.responseMimeType = application/json`.
    ResponseMimeType,
}

/// Owned rather than `&'static str` so the endpoint can be redirected — by tests to a
/// local stub, and by the Settings override that PR B adds.
#[derive(Debug, Clone)]
pub struct ProviderSpec {
    pub base_url: String,
    pub model_id: String,
    pub auth: AuthStyle,
    pub json_mode: JsonMode,
}

impl ProviderSpec {
    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    #[cfg(test)]
    pub fn with_model_id(mut self, model_id: &str) -> Self {
        self.model_id = model_id.to_string();
        self
    }
}

pub fn provider_spec(provider: LlmProvider) -> ProviderSpec {
    match provider {
        LlmProvider::ScalewayDeepseek => ProviderSpec {
            base_url: "https://api.scaleway.ai/v1".into(),
            // Default from Scaleway catalogue (2026-09); overridable later in Settings.
            model_id: "deepseek-v4-flash-0731".into(),
            auth: AuthStyle::Bearer,
            json_mode: JsonMode::Schema,
        },
        LlmProvider::Mistral => ProviderSpec {
            base_url: "https://api.mistral.ai/v1".into(),
            model_id: "mistral-small-latest".into(),
            auth: AuthStyle::Bearer,
            json_mode: JsonMode::JsonObject,
        },
        LlmProvider::Gemini => ProviderSpec {
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            model_id: "gemini-2.0-flash".into(),
            auth: AuthStyle::Header("x-goog-api-key"),
            json_mode: JsonMode::ResponseMimeType,
        },
    }
}

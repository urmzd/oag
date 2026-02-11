use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A security scheme type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecuritySchemeType {
    ApiKey,
    Http,
    OAuth2,
    OpenIdConnect,
    MutualTLS,
}

/// Location of an API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Query,
    Header,
    Cookie,
}

/// OAuth2 flows configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthFlows {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<OAuthFlow>,
    #[serde(rename = "clientCredentials", skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<OAuthFlow>,
    #[serde(rename = "authorizationCode", skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<OAuthFlow>,
}

/// A single OAuth2 flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthFlow {
    #[serde(rename = "authorizationUrl", skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(rename = "tokenUrl", skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    #[serde(default)]
    pub scopes: IndexMap<String, String>,
}

/// A security scheme definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: SecuritySchemeType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(rename = "in", skip_serializing_if = "Option::is_none")]
    pub location: Option<ApiKeyLocation>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,

    #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows: Option<OAuthFlows>,

    #[serde(rename = "openIdConnectUrl", skip_serializing_if = "Option::is_none")]
    pub open_id_connect_url: Option<String>,
}

/// A security requirement: map of scheme name → required scopes.
pub type SecurityRequirement = IndexMap<String, Vec<String>>;

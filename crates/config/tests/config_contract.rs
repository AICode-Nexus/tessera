use tessera_config::{ProviderProfile, TesseraConfig};

#[test]
fn config_loads_profiles_without_secret_values() {
    let toml = r#"
data_dir = "/tmp/tessera-test"

[[providers]]
id = "mock"
kind = "mock"
default_model = "mock-chat"
api_key_env = "MOCK_API_KEY"

[[providers]]
id = "local-ollama"
kind = "ollama"
default_model = "llama3.2"
base_url = "http://localhost:11434"
"#;

    let config: TesseraConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.data_dir.as_deref(), Some("/tmp/tessera-test"));
    assert_eq!(
        config.providers,
        vec![
            ProviderProfile {
                id: "mock".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                base_url: None,
                api_key_env: Some("MOCK_API_KEY".to_string()),
            },
            ProviderProfile {
                id: "local-ollama".to_string(),
                kind: "ollama".to_string(),
                default_model: "llama3.2".to_string(),
                base_url: Some("http://localhost:11434".to_string()),
                api_key_env: None,
            }
        ]
    );
}

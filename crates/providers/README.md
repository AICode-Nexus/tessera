# tessera-providers

Provider trait and provider adapters.

v0.1 includes a deterministic mock provider plus OpenAI-compatible and Ollama streaming adapter skeletons. Providers emit standard `RunEvent` values and safe capability metadata. The mock provider also supports a `mock-slow` model name for deterministic delayed streaming tests such as cancellation; it remains provider-neutral and does not execute tools.

This crate must not write storage, render UI, execute tools, decide policy, or own memory and agent behavior.

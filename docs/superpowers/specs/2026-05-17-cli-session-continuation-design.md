# CLI Session Continuation Design

## Goal

After `/resume <trace_id>`, the next prompt should continue with the restored transcript as provider-visible chat history instead of only restoring the visible CLI projection.

## Scope

This slice adds provider-neutral chat history plumbing:

- `ProviderRequest` carries ordered chat messages.
- `ConversationRequest` carries optional prior messages plus the current prompt.
- CLI REPL builds prior messages from `ClientSnapshot` before dispatching the next prompt.
- `/resume` remains read-only trace projection; it does not mutate old traces or re-run provider calls.

This does not add compaction, context file loading, tool execution, agent runtime, MCP runtime, or long-term memory.

## Architecture

`tessera-cli` owns the interactive transcript projection and converts visible user/assistant messages into prior conversation messages. `tessera-core` records only the current user message into the new trace while passing the full ordered message list to the provider adapter. Provider adapters serialize `messages` according to their protocol.

The fallback remains safe: if no prior messages exist, the provider request contains a single current user message, preserving one-shot behavior.

## Testing

- Provider request builders include prior user/assistant messages and the current user prompt in order.
- Core passes history to provider without writing the historical messages as new trace events.
- CLI `/resume` followed by a prompt sends restored messages as history.
- Existing one-shot and REPL smoke paths keep working.
